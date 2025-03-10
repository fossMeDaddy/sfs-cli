use std::{path::PathBuf, pin::Pin, sync::Arc};

use anyhow::anyhow;
use futures_util::Stream;
use orion::aead::streaming;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncSeekExt, sync::mpsc, task};

use crate::{
    constants, shared_types,
    state::STATE,
    utils::{self, streams::read_into_stream},
};

use super::{get_base_url, get_builder};

fn get_blob_upload_url() -> anyhow::Result<url::Url> {
    let mut url = get_base_url()?;
    url.set_path("/blob/upload");

    Ok(url)
}

#[derive(Default)]
pub struct ProgressStat {
    //pub id: Option<u32>,
    pub total_len: Option<u64>,
    pub increment: usize,
}

impl ProgressStat {
    //pub fn with_id(mut self, id: u32) -> Self {
    //    self.id = Some(id);
    //    self
    //}

    pub fn with_total_len(mut self, total_len: u64) -> Self {
        self.total_len = Some(total_len);
        self
    }

    pub fn with_increment(mut self, inc: usize) -> Self {
        self.increment = inc;
        self
    }
}

pub struct UploadFileOpts {
    pub upload_filepath: PathBuf,
    /// file is read and encrypted with this password (encryption: AES-GCM)
    pub password: Option<String>,
    pub is_zip_file: bool,
    pub file_stream_read_buf_size: u32,
}

impl<'a> UploadFileOpts {
    pub fn new(upload_filepath: PathBuf, password: Option<String>) -> Self {
        Self {
            upload_filepath,
            password,
            is_zip_file: false,
            file_stream_read_buf_size: constants::FILE_STREAM_READ_BUF_SIZE,
        }
    }

    fn new_encryptor(
        &self,
    ) -> anyhow::Result<Option<utils::crypto::CryptoStream<streaming::StreamSealer>>> {
        match &self.password {
            Some(p) => Ok(Some(utils::crypto::new_encryptor(p)?)),
            None => Ok(None),
        }
    }
}
/// uploads file in chunks if file is bigger than MIN_MULTIPART_UPLOAD_SIZE
pub async fn upload_file(
    mut upload_metadata: shared_types::UploadBlobMetadata,
    opts: &UploadFileOpts,
    progress_updater: mpsc::UnboundedSender<ProgressStat>,
) -> anyhow::Result<shared_types::FsFile> {
    let state = STATE.read().unwrap().clone();

    if state.get_active_token()?.is_none() {
        return Err(anyhow!(
            "no token selected! please login or add an access token to use."
        ));
    }

    let upload_file = fs::File::open(&opts.upload_filepath).await?;
    let upload_file_metadata = upload_file.metadata().await?;
    let enc_res = opts
        .new_encryptor()
        .map_err(|_| anyhow!("error occured while creating encryptor!"))?;

    upload_metadata.encryption = match enc_res.as_ref() {
        Some(s) => {
            match opts.is_zip_file {
                true => Some(shared_types::EncryptionMetadata::no_encryption()),
                false => {
                    if upload_file_metadata.len() >= constants::MP_CHUNK_SIZE {
                        return Err(anyhow!("currently, uploading encrypted files greater than {} is not supported!", constants::MP_CHUNK_SIZE));
                    }
                    Some(s.into_encryption_metadata(Some(opts.file_stream_read_buf_size)))
                }
            }
        }
        None => None,
    };

    let file: shared_types::FsFile;
    if upload_file_metadata.len() >= constants::MP_CHUNK_SIZE {
        let file_size = upload_file_metadata.len();
        let n_concurrent_chunks = 5;

        let upload_id = create_multipart_upload(&upload_metadata).await?;
        let upload_id = Arc::new(upload_id);

        let (sender, mut receiver) = mpsc::unbounded_channel::<usize>();

        let mut chunks_iter = 0..constants::MP_CHUNK_SIZE.div_ceil(file_size);
        let mut parts: Vec<UploadPartResult> = vec![];
        loop {
            let now_upload_chunks = chunks_iter
                .by_ref()
                .take(n_concurrent_chunks)
                .collect::<Vec<_>>();
            if now_upload_chunks.len() == 0 {
                break;
            }

            let mut concurrent_handles = vec![];
            for chunk_n in now_upload_chunks {
                let mut f = fs::File::open(&opts.upload_filepath).await?;
                _ = f
                    .seek(std::io::SeekFrom::Start(chunk_n * constants::MP_CHUNK_SIZE))
                    .await?;

                let upload_part_opts = UploadPartOpts {
                    stream: read_into_stream(
                        f,
                        constants::MP_CHUNK_SIZE as u32,
                        None,
                        Some(sender.clone()),
                    ),
                    part_num: (chunk_n + 1) as u32,
                    upload_id: upload_id.to_string(),
                };
                let h = upload_part(upload_part_opts);

                concurrent_handles.push(h);
            }

            let h = tokio::spawn(futures_util::future::join_all(concurrent_handles));
            while let Some(tic) = receiver.recv().await {
                _ = progress_updater.send(ProgressStat::default().with_increment(tic));
            }

            let res = h.await?;
            for r in res {
                parts.push(r?);
            }
        }

        let res = complete_multipart_upload(&parts).await?;
        file = res.file;
    } else {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let upload_metadata = Arc::new(upload_metadata);
        let file_stream_read_buf_size = opts.file_stream_read_buf_size;
        let upload_handle = task::spawn(async move {
            upload_blob_stream(
                utils::streams::read_into_stream(
                    upload_file,
                    file_stream_read_buf_size,
                    enc_res.map(|enc| enc.e),
                    Some(sender),
                ),
                upload_metadata.as_ref(),
            )
            .await
        });

        while let Some(v) = receiver.recv().await {
            progress_updater.send(
                ProgressStat::default()
                    .with_total_len(upload_file_metadata.len())
                    .with_increment(v),
            );
        }
        let fs_file = upload_handle.await??;
        drop(progress_updater);

        if opts.is_zip_file {
            if let Err(err) = fs::remove_file(opts.upload_filepath.clone()).await {
                println!(
                    "WARNING: failed to delete file '{}'\n{err}",
                    opts.upload_filepath.to_string_lossy()
                );
            };
        }

        file = fs_file;
    }

    Ok(file)
}

pub async fn upload_blob_stream(
    stream: Pin<Box<dyn Stream<Item = anyhow::Result<Vec<u8>>> + Send + 'static>>,
    upload_metadata: &shared_types::UploadBlobMetadata,
) -> anyhow::Result<shared_types::FsFile> {
    let url = get_blob_upload_url()?;

    let res = get_builder(reqwest::Method::POST, url)?
        .header(
            constants::HEADER_UPLOAD_METADATA,
            serde_json::to_string(upload_metadata)?,
        )
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await?;
    let status = res.status();

    if !status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow!(
            "({status}) error occured while uploading blob!\n{res_text}"
        ));
    }

    let res_data: shared_types::ApiResponse<shared_types::FsFile> = res.json().await?;
    res_data
        .data
        .ok_or(anyhow!("received null data from API response!"))
}

async fn create_multipart_upload(
    metadata: &shared_types::UploadBlobMetadata,
) -> anyhow::Result<String> {
    let mut url = get_base_url()?;
    url.set_path("/blob/create-multipart-upload");

    let metadata_b = serde_json::to_vec(&metadata)?;

    let metadata_part = reqwest::multipart::Part::bytes(metadata_b);
    let form = multipart::Form::new().part("metadata", metadata_part);
    let res = get_builder(reqwest::Method::POST, url.clone())?
        .multipart(form)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let res_text = res.text().await?;

        return Err(anyhow!(
            "({status}) error occured while creating multipart upload!\n{res_text}"
        ));
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResContent {
        upload_id: String,
    }
    let res_data: shared_types::ApiResponse<ResContent> = res.json().await?;
    res_data
        .data
        .ok_or(anyhow!("API response returned null data!"))
        .map(|data| data.upload_id)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadPartResult {
    pub part_number: u32,
    pub etag: String,
}
pub struct UploadPartOpts {
    pub upload_id: String,
    pub part_num: u32,
    pub stream: Pin<Box<dyn Stream<Item = anyhow::Result<Vec<u8>>> + Send + 'static>>,
}
pub async fn upload_part(opts: UploadPartOpts) -> anyhow::Result<UploadPartResult> {
    let mut url = get_base_url()?;
    url.set_path("/blob/upload-part");
    url.query_pairs_mut()
        .append_pair("id", &opts.upload_id)
        .append_pair("n", &opts.part_num.to_string());

    let res = get_builder(reqwest::Method::POST, url)?
        .body(reqwest::Body::wrap_stream(opts.stream))
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow!(
            "({status}) error occured while uploading part number {}!\n{res_text}",
            opts.part_num
        ));
    }

    let res_data: shared_types::ApiResponse<UploadPartResult> = res.json().await?;
    res_data
        .data
        .ok_or(anyhow!("received null data from API response!"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultipartUploadResult {
    //pub created: bool,
    pub file: shared_types::FsFile,
}

pub async fn complete_multipart_upload(
    parts: &Vec<UploadPartResult>,
) -> anyhow::Result<MultipartUploadResult> {
    let mut url = get_base_url()?;
    url.set_path("/blob/complete-multipart-upload");

    let res = get_builder(reqwest::Method::POST, url)?
        .json(parts)
        .send()
        .await?;

    let status = res.status();

    if !status.is_success() {
        let res_text = res.text().await?;

        return Err(anyhow!(
            "({status}) error occured in multipart upload completion!\n{}",
            res_text
        ));
    }

    let res_data: shared_types::ApiResponse<MultipartUploadResult> = res.json().await?;
    res_data
        .data
        .ok_or(anyhow!("received null data from API response!"))
}

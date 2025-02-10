use std::{path::PathBuf, pin::Pin, sync::Arc};

use anyhow::anyhow;
use futures_util::Stream;
use indicatif::ProgressBar;
use orion::aead::streaming;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::mpsc, task};

use crate::{
    constants,
    shared_types::{self, ApiResponse, FsFile, UploadBlobMetadata},
    state::STATE,
    utils,
};

use super::{get_base_url, get_builder};

fn get_blob_upload_url() -> anyhow::Result<url::Url> {
    let mut url = get_base_url()?;
    url.set_path("/blob/upload");

    Ok(url)
}

pub struct UploadFileOpts {
    pub upload_filepath: PathBuf,
    /// file is read and encrypted with this password (encryption: AES-GCM)
    pub password: Option<String>,
    pub is_zip_file: bool,
    pub file_stream_read_buf_size: u32,
    pub progress_bar: ProgressBar,
}

impl<'a> UploadFileOpts {
    pub fn new(
        upload_filepath: PathBuf,
        password: Option<String>,
        progress_bar: ProgressBar,
    ) -> Self {
        Self {
            upload_filepath,
            password,
            is_zip_file: false,
            file_stream_read_buf_size: constants::FILE_STREAM_READ_BUF_SIZE,
            progress_bar,
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
    mut upload_metadata: UploadBlobMetadata,
    opts: UploadFileOpts,
) -> anyhow::Result<FsFile> {
    let state = STATE.read().unwrap();

    if state.get_active_token()?.is_none() {
        return Err(anyhow!(
            "no token selected! please login or add an access token to use."
        ));
    }

    let upload_file = fs::File::open(opts.upload_filepath.clone()).await?;
    let file_metadata = upload_file.metadata().await?;

    //let mut file_buf = vec![0; file_metadata.len() as usize];
    //upload_file
    //    .read(&mut file_buf)
    //    .await
    //    .expect("reading file!");
    //if let Some(encrypter) = opts.new_encrypter() {
    //    upload_metadata.encryption = Some(shared_types::EncryptionMetadata {
    //        attempt_decryption: !opts.is_zip_file,
    //        salt: encrypter.salt.to_vec(),
    //        nonce: encrypter.nonce.to_vec(),
    //    });
    //    println!("FUCK YEAH, ENCRYPTION STARTING.");
    //    println!("before encryption size: {}", file_buf.len());
    //    file_buf = encrypter
    //        .encrypt_buffer(&file_buf)
    //        .expect("file buf ciphering failed!");
    //    println!("AFTER encryption size: {}", file_buf.len());
    //};
    //let mut url = get_base_url()?;
    //url.set_path("/blob/upload");
    //let form = reqwest::multipart::Form::new()
    //    .part(
    //        "file",
    //        reqwest::multipart::Part::bytes(file_buf).file_name(upload_metadata.name.clone()),
    //    )
    //    .part(
    //        "metadata",
    //        reqwest::multipart::Part::bytes(serde_json::to_vec(&upload_metadata).unwrap()),
    //    );
    //let res = get_builder(reqwest::Method::POST, url)?
    //    .multipart(form)
    //    .send()
    //    .await
    //    .expect("test file buf upload reqwest failed!");
    //let status = res.status();
    //if !status.is_success() {
    //    let res_text = res.text().await?;
    //    return Err(anyhow::anyhow!("FUCK! {res_text}"));
    //}
    //let res_data: ApiResponse<FsFile> = res.json().await?;
    //return res_data
    //    .data
    //    .ok_or(anyhow::anyhow!("WELL, FUCK. data received null"));

    let enc_res = opts
        .new_encryptor()
        .map_err(|_| anyhow!("error occured while creating encryptor!"))?;

    upload_metadata.encryption = match enc_res.as_ref() {
        Some(s) => match opts.is_zip_file {
            true => Some(shared_types::EncryptionMetadata::default_zipfile()),
            false => Some(s.into_encryption_metadata(Some(opts.file_stream_read_buf_size))),
        },
        None => None,
    };
    //.map(|encryptor| shared_types::EncryptionMetadata {
    //    attempt_decryption: !opts.is_zip_file,
    //    nonce: Some(encryptor.nonce.as_ref().to_vec()),
    //    salt: Some(encryptor.salt.as_ref().to_vec()),
    //    block_size: Some(file_stream_read_buf_size),
    //});
    println!("encryption metadata {:?}", upload_metadata.encryption);

    // TODO: implement chunky upload for non-encrypted files
    //
    //if file_metadata.len() >= constants::MIN_MULTIPART_UPLOAD_SIZE as u64 {
    //    let chunk_size = (file_metadata.len() as f32 / constants::N_MULTIPART_UPLOAD_CHUNKS as f32)
    //        .floor() as u64;
    //
    //    let upload = upload_file_in_chunks(chunk_size, enc_res, &upload_metadata, &opts).await?;
    //    file = upload.file;
    //}

    let (sender, mut receiver) = mpsc::unbounded_channel();

    let upload_metadata = Arc::new(upload_metadata);
    let upload_handle = task::spawn(async move {
        upload_blob_stream(
            utils::streams::read_into_stream(
                upload_file,
                opts.file_stream_read_buf_size,
                enc_res.map(|enc| enc.e),
                Some(sender),
            ),
            &upload_metadata,
        )
        .await
    });

    while let Some(v) = receiver.recv().await {
        opts.progress_bar.inc(v as u64);
    }
    let file = upload_handle.await??;
    opts.progress_bar.finish();

    if opts.is_zip_file {
        if let Err(err) = fs::remove_file(opts.upload_filepath.clone()).await {
            println!(
                "WARNING: failed to delete file '{}'\n{err}",
                opts.upload_filepath.to_string_lossy()
            );
        };
    }

    Ok(file)
}

pub async fn upload_blob_stream(
    stream: Pin<Box<dyn Stream<Item = anyhow::Result<Vec<u8>>> + Send + 'static>>,
    upload_metadata: &UploadBlobMetadata,
) -> anyhow::Result<FsFile> {
    let url = get_blob_upload_url()?;

    let file_part = reqwest::multipart::Part::stream(reqwest::Body::wrap_stream(stream))
        .file_name(upload_metadata.name.clone());
    let metadata_part = reqwest::multipart::Part::bytes(serde_json::to_vec(upload_metadata)?);
    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .part("metadata", metadata_part);

    let res = get_builder(reqwest::Method::POST, url)?
        .multipart(form)
        .send()
        .await?;
    let status = res.status();

    if !status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow!(
            "({status}) error occured while uploading blob!\n{res_text}"
        ));
    }

    let res_data: ApiResponse<FsFile> = res.json().await?;
    res_data
        .data
        .ok_or(anyhow!("received null data from API response!"))
}

// NOTE: need to check, will cloning encryptor like a basic bitch help it not randomly rotating
// keys and nonces between parallely uploading chunks?
// [DOESNT ALLOW CLONE]
// unpredicatable chunks order when encrypting, not good if key rotations, nonce increments take
// place for each encryption (probably just stick to uploading a single linear stream of encrypted
// bytes, DISALLOW MULTIPART when encryption is on)
// TODO: do this again, chunks might interefere
//
//async fn upload_file_in_chunks(
//    chunk_size: u64,
//    encryptor: Option<utils::crypto::Encryptor>,
//    upload_metadata: &UploadBlobMetadata,
//    opts: &UploadFileOpts<'_>,
//) -> anyhow::Result<MultipartUploadResult> {
//    let upload_id = create_multipart_upload(upload_metadata).await?;
//    let (enc_stream, header) = match encryptor {
//        Some((stream, header)) => (Some(stream), Some(header)),
//        None => (None, None),
//    };
//
//    let handles_iter = (0..constants::N_MULTIPART_UPLOAD_CHUNKS as u64).map(|n| {
//        let upload_filepath = opts.upload_filepath.clone();
//        let upload_id = upload_id.clone();
//
//        async {
//            let mut upload_file = fs::File::open(upload_filepath).await?;
//            upload_file.seek(SeekFrom::Start(n * chunk_size)).await?;
//
//            let upload_part_opts = UploadPartOpts {
//                upload_id: &upload_id,
//                part_num: n as usize,
//                stream: read_file_stream(
//                    upload_file,
//                    chunk_size,
//                    FILE_STREAM_READ_CHUNK_SIZE,
//                    enc_stream,
//                ),
//            };
//            upload_part(upload_part_opts).await
//        }
//    });
//
//    let mut parts = vec![];
//    for res_handle in futures_util::future::join_all(handles_iter).await {
//        let part = res_handle?;
//        parts.push(part);
//    }
//
//    complete_multipart_upload(&parts).await
//}

async fn create_multipart_upload(metadata: &UploadBlobMetadata) -> anyhow::Result<String> {
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
    let res_data: ApiResponse<ResContent> = res.json().await?;
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
pub struct UploadPartOpts<'a> {
    pub upload_id: &'a str,
    pub part_num: usize,
    pub stream: Pin<Box<dyn Stream<Item = anyhow::Result<Vec<u8>>> + Send + 'static>>,
}
pub async fn upload_part<'a>(opts: UploadPartOpts<'a>) -> anyhow::Result<UploadPartResult> {
    let mut url = get_base_url()?;
    url.set_path("/blob/upload-part");
    url.query_pairs_mut()
        .append_pair("id", opts.upload_id)
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

    let res_data: ApiResponse<UploadPartResult> = res.json().await?;
    res_data
        .data
        .ok_or(anyhow!("received null data from API response!"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultipartUploadResult {
    //pub created: bool,
    pub file: FsFile,
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

    let res_data: ApiResponse<MultipartUploadResult> = res.json().await?;
    res_data
        .data
        .ok_or(anyhow!("received null data from API response!"))
}

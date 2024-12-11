use anyhow::anyhow;
use reqwest::multipart;
use tokio::{fs, io::AsyncReadExt};

use crate::{
    config::CliConfig,
    constants,
    shared_types::{ApiResponse, FsFile, UploadSingleBlogMetadata},
    utils::crypto::Encrypter,
};

use super::{get_base_url, get_builder};

pub async fn upload_file(
    config: &CliConfig,
    upload_metadata: &UploadSingleBlogMetadata,
    upload_file: &mut fs::File,
    password: Option<&str>,
) -> anyhow::Result<FsFile> {
    let mut url = get_base_url(config)?;
    url.set_path("/blob/upload");

    let upload_metadata_str = serde_json::to_vec(&upload_metadata)?;

    let mut file_buf: Vec<u8> = Vec::new();
    upload_file.read_to_end(&mut file_buf).await?;

    if let Some(password) = password {
        let encrypter = Encrypter::new(password);
        file_buf = encrypter.encrypt_buffer(&file_buf)?;
    }

    let file_part = multipart::Part::bytes(file_buf)
        .file_name(upload_metadata.name.as_deref().unwrap().to_string())
        .mime_str(&upload_metadata.file_type)?;

    let metadata_part = multipart::Part::bytes(upload_metadata_str)
        .mime_str(constants::MIME_TYPES.get("json").unwrap())?;

    let form = multipart::Form::new()
        .part("metadata", metadata_part)
        .part("file", file_part);

    let res = get_builder(reqwest::Method::POST, url)?
        .multipart(form)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let res_str = res.text().await?;

        return Err(anyhow!(
            "Error occured while uploading! (HTTP ERROR: {})\nResponse: {}",
            status.to_string(),
            res_str
        ));
    }

    let data: ApiResponse<FsFile> = res.json().await?;
    Ok(data.data.unwrap())
}

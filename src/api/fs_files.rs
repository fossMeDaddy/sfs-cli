use clap::{Args, Parser, ValueEnum};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    config::CliConfig,
    shared_types::{ApiResponse, FsFile},
    utils::filters::parse_filter_str,
};

#[derive(Args)]
pub struct CliColFilters {
    #[arg(long)]
    pub deleted_at: Vec<String>, // "<,>,=2024-01-01 14:14:14" OR "2024-01-01 14:14:14...2024-01-01 16:14:14"

    #[arg(long)]
    pub created_at: Vec<String>,

    #[arg(long)]
    pub file_size: Vec<String>, // 25.0kb, 12mb, 0 (b)
}

impl CliColFilters {
    pub fn parse_get_filters(&self) -> anyhow::Result<Vec<Filter>> {
        let mut filters: Vec<Filter> = vec![];

        for filter_str in &self.deleted_at {
            filters.extend(parse_filter_str(FilterCol::DeletedAt, &filter_str)?);
        }
        for filter_str in &self.created_at {
            filters.extend(parse_filter_str(FilterCol::CreatedAt, &filter_str)?);
        }
        for filter_str in &self.file_size {
            filters.extend(parse_filter_str(FilterCol::FileSize, &filter_str)?);
        }

        Ok(filters)
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum FilterCol {
    CreatedAt,
    DeletedAt,
    FileType,
    FileSize,
    IsEncrypted,
    IsPublic,
    Name,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FilterOp {
    Gt,
    Lt,
    Eq,
    Ne,
}

#[derive(Serialize, Parser, Debug, ValueEnum, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum OrderCol {
    DeletedAt,
    CreatedAt,
    FileSize,
}

#[derive(Serialize, Parser, Debug, ValueEnum, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Serialize, Debug)]
pub struct Filter(pub FilterCol, pub FilterOp, pub serde_json::Value);

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetFilesOpts {
    pub filters: Option<Vec<Filter>>,
    pub limit: Option<usize>,
    pub page: Option<usize>,
    pub order_by: Option<OrderCol>,
    pub order: Option<Order>,
}

impl GetFilesOpts {
    pub fn new(filters: Option<Vec<Filter>>) -> Self {
        Self {
            filters,
            page: None,
            limit: None,
            order: None,
            order_by: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFilesReqBody {
    pub files: Vec<FsFile>,
    pub count: usize,
    pub page_size: usize,
}

pub async fn get_files(
    config: &CliConfig,
    dir_id: &str,
    opts: Option<GetFilesOpts>,
) -> anyhow::Result<GetFilesReqBody> {
    let mut url = super::get_base_url(config)?;

    url.set_path(&format!("fs/get-files/{}", dir_id));

    let mut req = super::get_builder(reqwest::Method::POST, url)?;
    if let Some(opts) = opts {
        req = req.json(&opts);
    }
    let res = req.send().await?;

    let status = res.status();
    if !status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow::anyhow!(
            "Request returned non-ok status code! ({})\nResponse: {}",
            status,
            res_text
        ));
    }

    let res_data: ApiResponse<GetFilesReqBody> = res.json().await?;
    match res_data.data {
        Some(data) => Ok(data),
        None => {
            return Err(anyhow::anyhow!(
                "Invalid response!\nMessage: {}\nError: {}",
                res_data.message,
                res_data.error.unwrap_or("".to_string())
            ))
        }
    }
}

/// provides successful response object to run `.bytes_stream()` on
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicFileMetadata {
    pub is_encrypted: bool,
    pub file_type: String,
    pub name: String,
}

pub async fn get_file_response(
    config: &CliConfig,
    storage_id: &str,
    token: Option<&str>,
) -> anyhow::Result<(PublicFileMetadata, Response)> {
    let mut url = super::get_base_url(config)?;

    url.set_path(storage_id);
    if let Some(token) = token {
        url.set_query(Some(&format!("token={}", token)));
    }

    let res = super::get_builder(reqwest::Method::GET, url)?
        .send()
        .await?;
    let res_status = res.status();
    if !res_status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow::anyhow!(
            "Error occured in request ({})\nResponse: {}",
            res_status,
            res_text
        ));
    }

    let public_file_metadata: PublicFileMetadata = match res.headers().get("metadata") {
        Some(value) => serde_json::from_str(value.to_str()?)?,
        None => return Err(anyhow::anyhow!("file metadata not found in response!")),
    };

    Ok((public_file_metadata, res))
}

pub async fn get_file_metadata(
    config: &CliConfig,
    storage_id: &str,
    token: Option<&str>,
) -> anyhow::Result<PublicFileMetadata> {
    let mut url = super::get_base_url(config)?;

    url.set_path(&format!("metadata/{}", storage_id));
    if let Some(token) = token {
        url.set_query(Some(&format!("token={}", token)));
    }

    let res = super::get_builder(reqwest::Method::GET, url)?
        .send()
        .await?;
    let res_status = res.status();
    if !res_status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow::anyhow!(
            "Error occured in request ({})\nResponse: {}",
            res_status,
            res_text
        ));
    }

    let res_data = res.json::<ApiResponse<PublicFileMetadata>>().await?;
    match res_data.data {
        Some(data) => Ok(data),
        None => {
            return Err(anyhow::anyhow!(
                "Invalid response!\nMessage: {}\nError: {}",
                res_data.message,
                res_data.error.unwrap_or("".to_string())
            ))
        }
    }
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetMetadata<'a> {
    pub storage_id: Option<&'a str>,
    pub is_public: Option<bool>,
    pub file_type: Option<&'a str>,
    pub name: Option<&'a str>,
}

pub async fn set_file_metadata(
    config: &CliConfig,
    metadata: SetMetadata<'_>,
) -> anyhow::Result<()> {
    let mut url = super::get_base_url(&config)?;
    url.set_path("/blob/set-metadata");

    let res = super::get_builder(reqwest::Method::POST, url)?
        .json(&metadata)
        .send()
        .await?;

    let res_status = res.status();
    if !res_status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow::anyhow!(
            "Response returned non-ok status code ({})\n{}",
            res_status,
            res_text
        ));
    }

    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFilesReqBody<'a> {
    pub dir_id: &'a str,
    pub file_names: &'a Vec<String>,
}

pub async fn delete_files(config: &CliConfig, opts: &DeleteFilesReqBody<'_>) -> anyhow::Result<()> {
    let mut url = super::get_base_url(config)?;
    url.set_path(&format!("/blob/delete"));

    let res = super::get_builder(reqwest::Method::POST, url)?
        .json(opts)
        .send()
        .await?;

    let res_status = res.status();
    if !res_status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow::anyhow!(
            "Api returned non-ok response ({})\nResponse: {}",
            res_status,
            res_text
        ));
    }

    Ok(())
}

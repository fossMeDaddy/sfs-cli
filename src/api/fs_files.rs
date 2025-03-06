use clap::{Args, Parser, ValueEnum};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    shared_types::{ApiResponse, FsFile},
    utils,
};

#[derive(Args)]
pub struct CliColFilters {
    #[arg(long)]
    /// format: "2024-01-01 14:14:14...2024-01-01 16:14:14" OR "...2024-01-01 16:14:14" OR "2024-01-01 16:14:14..."
    pub deleted_at: Vec<String>,

    #[arg(long)]
    /// format: "2024-01-01 14:14:14...2024-01-01 16:14:14" OR "...2024-01-01 16:14:14" OR "2024-01-01 16:14:14..."
    pub created_at: Vec<String>,

    #[arg(long)]
    /// format: "12kb...69.42mb" OR "...5mb" OR "512b..."
    pub file_size: Vec<String>,
}

impl CliColFilters {
    pub fn parse_get_filters(&self) -> anyhow::Result<Vec<Filter>> {
        let mut filters: Vec<Filter> = vec![];

        for filter_str in &self.deleted_at {
            filters.extend(utils::filters::parse_filter_str(
                FilterCol::DeletedAt,
                &filter_str,
            )?);
        }
        for filter_str in &self.created_at {
            filters.extend(utils::filters::parse_filter_str(
                FilterCol::CreatedAt,
                &filter_str,
            )?);
        }
        for filter_str in &self.file_size {
            filters.extend(utils::filters::parse_filter_str(
                FilterCol::FileSize,
                &filter_str,
            )?);
        }

        Ok(filters)
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum FilterCol {
    CreatedAt,
    DeletedAt,
    ContentType,
    Encryption,
    FileSize,
    IsPublic,
    Name,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FilterOp {
    Gt,
    Lt,
    Eq,
    Ne,
    IsNull,
    IsNotNull,
    Like,
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

#[derive(Serialize, Debug, Clone)]
pub struct Filter(pub FilterCol, pub FilterOp, pub serde_json::Value);

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FilterGroupType {
    And,
    Or,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FilterGroup {
    pub type_: FilterGroupType,
    pub filters: Vec<Filter>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetFilesOpts {
    pub dir_path: String,
    pub filters: Option<Vec<FilterGroup>>,
    pub limit: Option<usize>,
    pub page: Option<usize>,
    pub order_by: Option<OrderCol>,
    pub order: Option<Order>,
}

impl GetFilesOpts {
    /// takes in valid `dir_path`
    pub fn new(dir_path: String) -> Self {
        let filters: Vec<FilterGroup> = vec![];
        Self {
            dir_path,
            filters: Some(filters),
            page: None,
            limit: None,
            order: None,
            order_by: None,
        }
    }

    pub fn add_filter_group(&mut self, group: FilterGroup) {
        if let Some(f) = &mut self.filters {
            f.push(group);
        } else {
            self.filters = Some(vec![group]);
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

pub async fn get_files(opts: Option<GetFilesOpts>) -> anyhow::Result<GetFilesReqBody> {
    let mut url = super::get_base_url()?;

    url.set_path(&format!("fs/get-files"));

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

pub async fn get_file(abs_filepath: &str) -> anyhow::Result<Option<FsFile>> {
    let (dirpath, filename) = utils::dirtree::split_path(abs_filepath);

    let filters = vec![Filter(FilterCol::Name, FilterOp::Eq, filename.into())];
    let mut opts = GetFilesOpts::new(dirpath.to_string());
    opts.add_filter_group(FilterGroup {
        type_: FilterGroupType::And,
        filters,
    });

    let mut res_files = get_files(Some(opts)).await?;

    Ok(res_files.files.pop())
}

pub async fn get_file_response(
    storage_id: &str,
    token: Option<&str>,
) -> anyhow::Result<(FsFile, Response)> {
    let mut url = super::get_base_url()?;

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

    let public_file_metadata: FsFile = match res.headers().get("metadata") {
        Some(value) => serde_json::from_str(value.to_str()?)?,
        None => return Err(anyhow::anyhow!("file metadata not found in response!")),
    };

    Ok((public_file_metadata, res))
}

pub async fn get_file_metadata(storage_id: &str, token: Option<&str>) -> anyhow::Result<FsFile> {
    let mut url = super::get_base_url()?;

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

    let res_data = res.json::<ApiResponse<FsFile>>().await?;
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
    pub path: &'a str,
    pub is_public: Option<bool>,
    pub cache_max_age_seconds: Option<u64>,
    pub name: Option<&'a str>,
}

pub async fn set_file_metadata(metadata: SetMetadata<'_>) -> anyhow::Result<()> {
    let mut url = super::get_base_url()?;
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
    pub dir_path: &'a str,
    pub file_names: &'a Vec<String>,
}

pub async fn delete_files(opts: &DeleteFilesReqBody<'_>) -> anyhow::Result<Vec<FsFile>> {
    let mut url = super::get_base_url()?;
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

    let res_data: ApiResponse<Vec<FsFile>> = res.json().await?;
    let data = res_data
        .data
        .ok_or(anyhow::anyhow!("'data' came null in api response!"))?;

    Ok(data)
}

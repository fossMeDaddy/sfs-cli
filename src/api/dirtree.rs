use std::collections::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::shared_types::{ApiResponse, AppContext, DirTree, FsFile};

use super::get_base_url;

pub struct DirTreeResponse {
    pub dirtree: DirTree,
    pub file_counts: HashMap<String, u32>,
}

pub async fn get_dirtree(ctx: &AppContext<'_>) -> anyhow::Result<DirTreeResponse> {
    let mut url = super::get_base_url(ctx)?;
    url.set_path("fs/tree");

    let res = super::get_builder(reqwest::Method::GET, url)?
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow::anyhow!(
            "{}, Error occured while fetching directory tree!\nResponse: {}",
            status,
            res_text
        ));
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResDataFileCountObj {
        dir_id: String,
        count: u32,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResData {
        dir_tree: DirTree,
        file_counts: Vec<ResDataFileCountObj>,
    }

    let res_data: ApiResponse<ResData> = res.json().await?;
    let data = match res_data.data {
        Some(data) => data,
        None => return Err(anyhow::anyhow!("No data found in response!")),
    };

    let dirtree = data.dir_tree;
    let file_counts: HashMap<String, u32> = data
        .file_counts
        .into_iter()
        .map(|obj| (obj.dir_id, obj.count))
        .collect();

    Ok(DirTreeResponse {
        dirtree,
        file_counts,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MvOpts<'a> {
    pub file_path: &'a str,
    pub new_file_path: &'a str,
}
pub async fn mv(ctx: &AppContext<'_>, opts: &MvOpts<'_>) -> anyhow::Result<FsFile> {
    let mut url = get_base_url(ctx)?;
    url.set_path("/fs/mv");

    let res = super::get_builder(reqwest::Method::POST, url)?
        .json(opts)
        .send()
        .await?;

    let res_status = res.status();
    if !res_status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow!("Error occured while moving file!\n{}", res_text));
    }

    let res_data: ApiResponse<FsFile> = res.json().await?;
    match res_data.data {
        Some(data) => Ok(data),
        None => return Err(anyhow!("Response returned no data!")),
    }
}

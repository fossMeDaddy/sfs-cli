use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    config::CliConfig,
    shared_types::{ApiResponse, DirTree},
};

pub struct DirTreeResponse {
    pub dirtree: DirTree,
    pub file_counts: HashMap<String, u32>,
}

pub async fn get_dirtree(config: &CliConfig) -> anyhow::Result<DirTreeResponse> {
    let mut url = super::get_base_url(config)?;
    url.set_path("fs/tree");

    let res = super::get_builder(reqwest::Method::GET, url)?
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow::anyhow!(
            "{}, Error occured while fetching directory tree!",
            res.status()
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

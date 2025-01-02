use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::shared_types::{ApiKey, ApiResponse, AppContext};

use super::{get_base_url, get_builder};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResData {
    pub access_token: String,
    pub api_key: ApiKey,
}
pub async fn login(ctx: &AppContext<'_>, key: &str) -> anyhow::Result<ResData> {
    let mut url = get_base_url(ctx)?;
    url.set_path("auth/gh-cli-login");
    url.query_pairs_mut().append_pair("key", key);

    let res = get_builder(reqwest::Method::GET, url)?.send().await?;
    let status = res.status();
    if !status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow!(
            "{0} ({status})\nResponse: {res_text}",
            status
                .canonical_reason()
                .unwrap_or("Unexpected error occured")
        ));
    }

    let res_data: ApiResponse<ResData> = res.json().await?;
    match res_data.data {
        Some(data) => Ok(data),
        None => {
            return Err(anyhow!(
                "'data' is not present!\nResponse: {}",
                serde_json::to_string_pretty(&res_data)?
            ))
        }
    }
}

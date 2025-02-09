use anyhow::anyhow;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    api::get_sudo_builder,
    shared_types::{ApiResponse, DirTree},
};

use super::get_base_url;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenAccessTokenRes {
    pub access_token: String,
    pub dirtree: DirTree,
}

pub async fn generate_access_token(
    acpl: &[String],
    expires_at: &DateTime<Utc>,
) -> anyhow::Result<GenAccessTokenRes> {
    let mut url = super::get_base_url()?;
    url.set_path("access/generate-token");

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Req<'a> {
        acpl: &'a [String],
        expires_at: &'a String,
    }

    let res = get_sudo_builder(reqwest::Method::POST, url)?
        .json(&Req {
            acpl,
            expires_at: &expires_at
                .to_utc()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        })
        .send()
        .await?;

    if !res.status().is_success() {
        let res_text = res.text().await?;
        return Err(anyhow::anyhow!(
            "Failed to generate access token!\nResponse: {}",
            res_text
        ));
    }

    let res_data: ApiResponse<GenAccessTokenRes> = res.json().await?;
    match res_data.data {
        Some(data) => Ok(data),
        None => Err(anyhow::anyhow!(
            "Response data is null! {}",
            serde_json::to_string_pretty(&res_data)?
        )),
    }
}

pub async fn blacklist_token(tokens: &Vec<String>) -> anyhow::Result<()> {
    let mut url = get_base_url()?;
    url.set_path("/access/blacklist-token");

    #[derive(Serialize)]
    struct ReqBody<'a> {
        tokens: &'a Vec<String>,
    }

    let res = get_sudo_builder(reqwest::Method::POST, url)?
        .json(&ReqBody { tokens })
        .send()
        .await?;
    let status = res.status();
    if !status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow!("API error occured: {status}\n{res_text}"));
    }

    Ok(())
}

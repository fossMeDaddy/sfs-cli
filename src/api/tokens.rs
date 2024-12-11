use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use url::Url;

use crate::{
    api::get_sudo_builder,
    config::CliConfig,
    shared_types::{AccessToken, ApiResponse},
};

pub async fn generate_access_token(
    config: &CliConfig,
    acpl: &Vec<String>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<AccessToken> {
    let mut url = Url::parse(config.get_base_url())?;
    url.set_path("access/generate-token");

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Req {
        acpl: Vec<String>,
        expires_at: String,
    }

    let res = get_sudo_builder(reqwest::Method::POST, url)?
        .json(&Req {
            acpl: acpl.clone(),
            expires_at: expires_at
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

    let res_data: ApiResponse<AccessToken> = res.json().await?;
    match res_data.data {
        Some(data) => Ok(data),
        None => Err(anyhow::anyhow!("Response data is null!")),
    }
}

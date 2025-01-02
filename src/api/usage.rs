use anyhow::anyhow;

use crate::shared_types::{ApiKeyUsage, ApiResponse, AppContext};

use super::get_sudo_builder;

pub async fn get_api_usage(ctx: &AppContext<'_>) -> anyhow::Result<ApiKeyUsage> {
    let mut url = super::get_base_url(ctx)?;
    url.set_path("/usage");

    let res = get_sudo_builder(ctx, reqwest::Method::GET, url)?
        .send()
        .await?;
    let res_status = res.status();

    if !res_status.is_success() {
        let res_text = res.text().await?;
        return Err(anyhow!("({res_status}): {res_text}"));
    }

    let res_data: ApiResponse<ApiKeyUsage> = res.json().await?;
    match res_data.data {
        Some(data) => Ok(data),
        None => return Err(anyhow!("Data was returned null!")),
    }
}

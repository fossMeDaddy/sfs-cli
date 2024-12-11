use base64::prelude::*;
use url::Url;

use crate::{config::CliConfig, utils::local_auth::LocalAuthData};

pub mod dirtree;
pub mod fs_files;
pub mod tokens;
pub mod uploads;

pub fn get_base_url(config: &CliConfig) -> anyhow::Result<Url> {
    let mut url = Url::parse(config.get_base_url())?;
    if let Some(auth_data) = LocalAuthData::get()? {
        url.set_query(Some(
            format!("token={}", auth_data.access_token.token).as_str(),
        ));
    }

    Ok(url)
}

pub fn get_builder(method: reqwest::Method, url: Url) -> anyhow::Result<reqwest::RequestBuilder> {
    let client = reqwest::Client::new();
    Ok(client.request(method, url))
}

pub fn get_sudo_builder(
    method: reqwest::Method,
    url: Url,
) -> anyhow::Result<reqwest::RequestBuilder> {
    let client = reqwest::Client::new();

    let auth_data = match LocalAuthData::get()? {
        Some(data) => data,
        None => return Err(anyhow::anyhow!("Please login first!")),
    };

    let api_creds = BASE64_STANDARD.encode(format!(
        "{}:{}",
        auth_data.api_key.key, auth_data.api_key.secret
    ));

    Ok(client
        .request(method, url)
        .header("Authorization", format!("Bearer {}", api_creds)))
}

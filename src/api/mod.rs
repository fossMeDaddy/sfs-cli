use anyhow::anyhow;
use base64::prelude::*;
use url::Url;

use crate::{
    config::CONFIG,
    state::{ActiveToken, STATE},
    utils::local_auth::LocalAuthData,
};

pub mod auth;
pub mod dirtree;
pub mod fs_files;
pub mod tokens;
pub mod uploads;
pub mod usage;
pub mod utils;

pub fn get_base_url() -> anyhow::Result<Url> {
    let config = CONFIG.read().unwrap();
    let state = STATE.read().unwrap();

    let mut url = Url::parse(config.get_base_url())?;
    if let Some((access_token, _)) = state.get_active_token()? {
        url.set_query(Some(format!("token={}", access_token).as_str()));
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
    let state = STATE.read().unwrap();

    let client = reqwest::Client::new();

    match state.active_token {
        ActiveToken::RootAccessToken => {}
        _ => return Err(anyhow!("only FileSystem owners are allowed to perform this action! either switch to your access token or ask the owner to perform this action for you."))
    };

    let auth_data =
        LocalAuthData::get().ok_or(anyhow!("Authentication not done! please login."))?;

    let api_creds = BASE64_STANDARD.encode(format!(
        "{}:{}",
        auth_data.api_key.key, auth_data.api_key.secret
    ));

    Ok(client
        .request(method, url)
        .header("Authorization", format!("Bearer {}", api_creds)))
}

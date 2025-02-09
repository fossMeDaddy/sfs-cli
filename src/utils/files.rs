use url::Url;

use crate::config::CONFIG;

/// does not validate anything, a dumb function to display a string
pub fn get_share_url(token: Option<&str>, storage_id: &str) -> anyhow::Result<Url> {
    let config = CONFIG.read().unwrap();

    let mut url = Url::parse(config.get_base_url())?;
    url.set_path(storage_id);
    if let Some(token) = token {
        url.set_query(Some(format!("token={}", token)).as_deref());
    }

    Ok(url)
}

pub fn get_file_ext<'a>(filename: &'a str) -> &'a str {
    filename.split(".").last().unwrap_or("bin")
}

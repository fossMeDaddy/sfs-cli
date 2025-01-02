use url::Url;

use crate::shared_types::AppContext;

/// does not validate anything, a dumb function to display a string
pub fn get_share_url(
    token: Option<&str>,
    storage_id: &str,
    ctx: &AppContext,
) -> anyhow::Result<Url> {
    let mut url = Url::parse(ctx.config.get_base_url())?;
    url.set_path(storage_id);
    if let Some(token) = token {
        url.set_query(Some(format!("token={}", token)).as_deref());
    }

    Ok(url)
}

pub fn get_file_ext<'a>(filename: &'a str) -> &'a str {
    filename.split(".").last().unwrap_or("bin")
}

pub fn get_pretty_size(filesize: usize) -> String {
    let mut i = 0;
    let mut size = filesize as f32;
    while i < 4 {
        let s = size / 10_f32.powi(3);
        if s < 1.0 {
            break;
        }

        size = s;
        i += 1;
    }

    match i {
        0 => format!("{:.3}b", size),
        1 => format!("{:.3}kb", size),
        2 => format!("{:.3}mb", size),
        3 => format!("{:.3}gb", size),
        _ => format!("{:.3}tb", size),
    }
}

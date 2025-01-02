use chrono::{DateTime, Local};
use serde::Deserialize;

use crate::{constants, utils::files};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FsFile {
    pub name: String,
    pub storage_id: String,
    pub cache_max_age_seconds: u64,
    pub file_system_id: String,
    pub dir_id: String,
    pub file_size: usize,
    pub is_encrypted: bool,
    pub is_public: bool,
    pub created_at: DateTime<Local>,
    pub deleted_at: Option<DateTime<Local>>,
}

impl FsFile {
    pub fn get_filetype(&self) -> &'static str {
        let mimetype = constants::MIME_TYPES.get(files::get_file_ext(&self.name));
        *mimetype.unwrap_or(&constants::UNKNOWN_MIME_TYPE)
    }
}

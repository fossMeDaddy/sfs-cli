use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

mod dirtree;
mod fs_files;

pub use dirtree::*;
pub use fs_files::*;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessToken {
    pub token: String,
    pub user_id: String,
    pub api_key: String,
    pub acpl: String,
    pub created_at: DateTime<Local>,
    pub expires_at: DateTime<Local>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    pub key: String,
    pub secret: String,
    pub user_id: String,
    pub free_reads: u32,
    pub free_writes: u32,
    pub free_storage_gb: u32,
    pub free_quota_interval_seconds: u32,
    pub storage_gb: u32,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

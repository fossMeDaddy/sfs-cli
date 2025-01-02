use std::str::FromStr;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

mod dirtree;
mod fs_files;

pub use dirtree::*;
pub use fs_files::*;

use crate::utils::str2x;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessToken {
    pub api_key: Option<String>,
    pub acpl: Vec<String>,
    pub expires_at: DateTime<Local>,
}

impl FromStr for AccessToken {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        str2x::str2at(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    pub key: String,
    pub secret: String,
    pub user_id: String,
    pub reads_limit: u64,
    pub writes_limit: u64,
    pub storage_gb_hour_limit: u64,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

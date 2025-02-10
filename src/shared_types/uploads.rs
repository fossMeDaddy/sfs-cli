use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionMetadata {
    pub attempt_decryption: bool,
    pub nonce: Option<Vec<u8>>,
    pub salt: Option<Vec<u8>>,

    /// bytes read, encrypted and uploaded at once (the read buffer size)
    ///
    /// needed for buffered decryption of incoming stream
    pub block_size: Option<u32>,
}
impl EncryptionMetadata {
    pub fn default_zipfile() -> Self {
        Self {
            attempt_decryption: false,
            nonce: None,
            salt: None,
            block_size: None,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UploadBlobMetadata {
    pub name: String,
    pub content_type: Option<String>,
    pub is_public: bool,
    pub encryption: Option<EncryptionMetadata>,
    pub cache_max_age_seconds: Option<u64>,
    pub dir_path: String,
    pub force_write: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadSingleBlogMetadata {
    pub is_public: bool,
    pub is_encrypted: bool,
    pub cache_max_age_seconds: Option<u64>,
    pub dir_path: String,
    pub name: Option<String>,
    pub file_type: String,
    pub force_write: bool,
}

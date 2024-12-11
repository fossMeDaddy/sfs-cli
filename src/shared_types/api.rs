use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadSingleBlogMetadata {
    pub is_public: bool,
    pub is_encrypted: bool,
    pub file_type: String,
    pub dir_path: String,
    pub name: Option<String>,
    pub force_write: bool,
}

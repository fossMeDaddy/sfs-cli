use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyUsage {
    pub reads_limit: u64,
    pub writes_limit: u64,
    pub storage_gb_hour_limit: f32,
    pub storage_gb_hour_used: f32,
    pub storage_gb_used: f32,
    pub reads_used: u64,
    pub writes_used: u64,
}

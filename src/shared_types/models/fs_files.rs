use chrono::{DateTime, Local};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FsFile {
    pub name: String,
    pub storage_id: String,
    pub file_system_id: String,
    pub dir_id: String,
    pub file_size: usize,
    pub file_type: String,
    pub is_encrypted: bool,
    pub is_public: bool,
    pub created_at: DateTime<Local>,
    pub deleted_at: Option<DateTime<Local>>,
}

impl FsFile {
    pub fn get_pretty_size(&self) -> String {
        let mut i = 0;
        let mut size = self.file_size as f32;
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
}

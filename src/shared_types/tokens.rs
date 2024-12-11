use std::fmt::Display;

use serde::Serialize;

#[derive(Serialize)]
pub enum AccessTokenPermission {
    ReadPrivate,
    FilesAdmin,
    FilesOwner,
    DirAdmin,
}

impl Display for AccessTokenPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessTokenPermission::ReadPrivate => write!(f, "read_private"),
            AccessTokenPermission::FilesAdmin => write!(f, "files_admin"),
            AccessTokenPermission::FilesOwner => write!(f, "files_owner"),
            AccessTokenPermission::DirAdmin => write!(f, "dir_admin"),
        }
    }
}

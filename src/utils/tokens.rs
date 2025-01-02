use crate::shared_types::AccessTokenPermission;

pub fn get_acp(permission: AccessTokenPermission, path_pattern: &str) -> String {
    format!("{}:{}", permission.to_string(), path_pattern)
}

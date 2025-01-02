use std::{fmt::Display, str::FromStr};

use anyhow::anyhow;
use serde::Serialize;

pub enum PermissionChar {
    Create,
    Read,
    Update,
    Delete,
}

impl Display for PermissionChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionChar::Create => write!(f, "c"),
            PermissionChar::Read => write!(f, "r"),
            PermissionChar::Update => write!(f, "u"),
            PermissionChar::Delete => write!(f, "d"),
        }
    }
}

impl FromStr for PermissionChar {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "c" => Ok(PermissionChar::Create),
            "r" => Ok(PermissionChar::Read),
            "u" => Ok(PermissionChar::Update),
            "d" => Ok(PermissionChar::Delete),
            _ => return Err(anyhow!("Invalid permission character.")),
        }
    }
}

#[derive(Serialize)]
pub struct AccessTokenPermission {
    perm_str: String,
}

impl Display for AccessTokenPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        _ = f.write_str(&self.perm_str);

        Ok(())
    }
}

impl FromStr for AccessTokenPermission {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 1 || s.len() > 4 {
            return Err(anyhow!("Invalid AccessControlPath permission length."));
        }

        let mut perm_str = String::new();
        for char in s.chars() {
            let c: PermissionChar = char.to_string().parse()?;
            perm_str += &c.to_string();
        }

        Ok(Self { perm_str })
    }
}

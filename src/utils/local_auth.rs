use anyhow;
use serde::{Deserialize, Serialize};

use crate::shared_types::{AccessToken, ApiKey};

#[derive(Serialize, Deserialize)]
pub struct LocalAuthData {
    pub access_token: AccessToken,
    pub api_key: ApiKey,
}

impl LocalAuthData {
    pub fn save(&self) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(env!("CARGO_PKG_NAME"), "login")?;
        let enc = serde_json::to_string(self)?;
        entry.set_secret(enc.as_bytes())?;

        Ok(())
    }

    pub fn get() -> anyhow::Result<Option<Self>> {
        let entry = keyring::Entry::new(env!("CARGO_PKG_NAME"), "login")?;
        let json_data = match entry.get_secret() {
            Ok(data) => data,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(err) => return Err(anyhow::Error::new(err)),
        };

        Ok(serde_json::from_slice(&json_data)?)
    }

    pub fn delete() -> keyring::Result<()> {
        let entry = keyring::Entry::new(env!("CARGO_PKG_NAME"), "login")?;
        entry.delete_credential()
    }
}

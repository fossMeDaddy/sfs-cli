use std::sync::Mutex;

use anyhow;
use serde::{Deserialize, Serialize};

use crate::{constants, shared_types::ApiKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAuthData {
    pub access_token: String,
    pub api_key: ApiKey,
}

impl LocalAuthData {
    pub fn save(&self) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(env!("CARGO_PKG_NAME"), constants::ROOT_ACCESS_TOKEN_TAG)?;
        let enc = serde_json::to_string(self)?;

        entry.set_secret(enc.as_bytes())?;
        Self::load()?;

        Ok(())
    }

    pub fn load() -> anyhow::Result<()> {
        let mut local_auth_data = LOCAL_AUTH_DATA.lock().unwrap();

        let entry = keyring::Entry::new(env!("CARGO_PKG_NAME"), constants::ROOT_ACCESS_TOKEN_TAG)?;
        let json_data = match entry.get_secret() {
            Ok(data) => data,
            Err(keyring::Error::NoEntry) => return Ok(()),
            Err(err) => return Err(anyhow::Error::new(err)),
        };

        let auth_data: LocalAuthData = serde_json::from_slice(&json_data)?;

        *local_auth_data = Some(auth_data);

        Ok(())
    }

    pub fn get() -> Option<Self> {
        let local_auth_data = LOCAL_AUTH_DATA.lock().unwrap();

        let auth_data = local_auth_data.as_ref();

        auth_data.cloned()
    }

    pub fn delete() -> anyhow::Result<()> {
        let entry = keyring::Entry::new(env!("CARGO_PKG_NAME"), constants::ROOT_ACCESS_TOKEN_TAG)?;

        entry.delete_credential()?;
        Self::load()?;

        Ok(())
    }
}

static LOCAL_AUTH_DATA: Mutex<Option<LocalAuthData>> = Mutex::new(None);

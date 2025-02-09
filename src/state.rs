use std::{
    collections::HashMap,
    fmt::Display,
    fs, io,
    path::PathBuf,
    sync::{Arc, LazyLock, RwLock},
};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    constants,
    shared_types::AccessToken,
    utils::{local_auth::LocalAuthData, paths::get_absolute_path},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActiveToken {
    RootAccessToken,
    Tag(String),
}

impl Display for ActiveToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tag(tagname) => {
                write!(f, "Tag({})", tagname)
            }
            Self::RootAccessToken => {
                write!(f, "RootAccessToken")
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentState {
    /// (tag, token)
    pub tokens: HashMap<String, String>,
    pub active_token: ActiveToken,
    pub working_directory: String,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            active_token: ActiveToken::RootAccessToken,
            tokens: HashMap::new(),
            working_directory: "/".to_string(),
        }
    }
}

impl PersistentState {
    pub fn get_state_filepath() -> io::Result<PathBuf> {
        get_absolute_path("~/.sfs/state.json")
    }

    pub fn get_wd(&self) -> &str {
        self.working_directory.as_str()
    }

    pub fn set_wd(&mut self, wd: &str) -> anyhow::Result<()> {
        self.guard_mutate(|s| {
            s.working_directory = if wd != "/" {
                wd.trim_end_matches("/").to_string()
            } else {
                wd.to_string()
            };

            Ok(())
        })
    }

    pub fn get_untitled_token_tag(&self) -> String {
        let mut counter = 0;
        for k in self.tokens.keys() {
            if k.starts_with(constants::UNTITLED_TAG_PREFX) {
                counter += 1;
            }
        }

        return format!("{}{}", constants::UNTITLED_TAG_PREFX, counter + 1);
    }

    pub fn get_active_token(&self) -> anyhow::Result<Option<(String, AccessToken)>> {
        match &self.active_token {
            ActiveToken::RootAccessToken => {
                let local_auth = match LocalAuthData::get() {
                    Some(auth_data) => auth_data,
                    None => return Ok(None),
                };

                let access_token: AccessToken = local_auth.access_token.parse()?;
                Ok(Some((local_auth.access_token, access_token)))
            }
            ActiveToken::Tag(tagname) => {
                let token = self
                    .tokens
                    .get(tagname)
                    .ok_or(anyhow!("token tag '{tagname}' not found!"))?;

                let parsed_access_token: AccessToken = token.parse()?;

                Ok(Some((token.to_string(), parsed_access_token)))
            }
        }
    }

    pub fn load() -> anyhow::Result<Self> {
        let contents = match fs::read_to_string(Self::get_state_filepath()?) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let state_filepath = Self::get_state_filepath()?;
                let default_contents = serde_json::to_vec_pretty(&PersistentState::default())?;

                fs::DirBuilder::new().recursive(true).create(
                    state_filepath
                        .parent()
                        .expect("ERROR! invalid default state file path '{state_filepath}'"),
                )?;
                fs::write(state_filepath, &default_contents)?;

                String::from_utf8(default_contents)?
            }
            Err(err) => return Err(anyhow!(err)),
        };
        let filestate: Self = serde_json::from_str(&contents)?;

        Ok(filestate)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let contents = serde_json::to_vec_pretty(self)?;
        fs::write(Self::get_state_filepath()?, contents)?;

        Ok(())
    }

    pub fn guard_mutate<F>(&mut self, mut f: F) -> anyhow::Result<()>
    where
        F: FnMut(&mut Self) -> anyhow::Result<()>,
    {
        f(self)?;
        _ = self.save();

        Ok(())
    }
}

pub static STATE: LazyLock<RwLock<PersistentState>> = LazyLock::new(|| {
    PersistentState::load()
        .expect("local state initialization errored out!")
        .into()
});

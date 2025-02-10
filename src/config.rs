use clap::ValueEnum;
use std::{
    fs, io, path,
    sync::{Arc, LazyLock, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::utils::paths::get_absolute_path;

#[derive(Serialize, Deserialize, PartialEq, Clone, ValueEnum)]
pub enum LogLevel {
    Chirpy,
    Normal,
    Stfu,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Chirpy
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct CliConfig {
    #[serde(skip_serializing_if = "is_default")]
    base_url: String,

    #[serde(skip_serializing_if = "is_default")]
    github_client_id: String,

    #[serde(skip_serializing_if = "is_default")]
    log_level: LogLevel,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.simplefs.io".to_string(),
            github_client_id: "Ov23li5rbT6pIfVXc7Rv".to_string(),
            log_level: LogLevel::Chirpy,
        }
    }
}

impl CliConfig {
    pub fn get_config_filepath() -> path::PathBuf {
        get_absolute_path("~/.sfs/config.toml").unwrap()
    }

    fn save_to_file(&self) -> anyhow::Result<()> {
        let config_filepath = CliConfig::get_config_filepath();
        fs::DirBuilder::new()
            .recursive(true)
            .create(config_filepath.parent().unwrap())?;

        let toml_str = toml::to_string(&self)?;
        fs::write(config_filepath, toml_str)?;

        Ok(())
    }

    pub fn get_gh_login_uri(&self) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}/auth/gh-callback&scope=repo,user,user:email",
            self.github_client_id,
            self.base_url,
        )
    }

    pub fn get_base_url(&self) -> &str {
        self.base_url.as_str()
    }

    pub fn get_log_level(&self) -> LogLevel {
        self.log_level.clone()
    }

    pub fn set_log_level(&mut self, log_level: LogLevel) -> anyhow::Result<()> {
        self.log_level = log_level;

        self.save_to_file()?;

        Ok(())
    }
}

pub static CONFIG: LazyLock<RwLock<CliConfig>> = LazyLock::new(|| {
    let config_filepath = CliConfig::get_config_filepath();
    let file_contents = match fs::read_to_string(config_filepath) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            let default_config = CliConfig::default();
            default_config
                .save_to_file()
                .expect("error occured while saving default config to file");
            return default_config.into();
        }
        Err(err) => panic!("{}", err),
    };

    toml::from_str(&file_contents).unwrap()
});

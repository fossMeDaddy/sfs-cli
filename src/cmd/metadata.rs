use chrono::Duration;
use clap::Parser;
use colored::Colorize;

use crate::{
    api::fs_files::{set_file_metadata, SetMetadata},
    shared_types::{self, CliSubCmd},
    state::STATE,
    utils::{dirtree, str2x},
};

#[derive(Parser)]
pub struct MetadataCommand {
    /// name of the file
    filename: String,

    #[arg(long)]
    /// the directory path the file is present in (not needed in usual cases, uses the WD set by default)
    dirpath: Option<String>,

    #[arg(long)]
    /// set a new name to set the file
    set_name: Option<String>,

    #[arg(long)]
    /// make the file public or private
    visibility: Option<shared_types::CmdVisibility>,

    #[arg(long, value_parser = str2x::str2duration)]
    /// set the 'max-age' value for cache, defaults to 0 (format: 12d23h34m45s)
    max_age: Option<Duration>,
}

impl CliSubCmd for MetadataCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let dirpath = match &self.dirpath {
            Some(dirpath) => dirtree::get_absolute_path(&dirpath, wd),
            None => wd.to_string(),
        };
        let path = format!("{}/{}", dirpath, &self.filename);

        let metadata = SetMetadata {
            path: &path,
            name: self.set_name.as_deref(),
            is_public: match self.visibility {
                Some(shared_types::CmdVisibility::Public) => Some(true),
                Some(shared_types::CmdVisibility::Private) => Some(false),
                None => None,
            },
            cache_max_age_seconds: match self.max_age {
                Some(max_age) => Some(max_age.num_seconds().abs() as u64),
                None => None,
            },
        };
        set_file_metadata(metadata)
            .await
            .expect("error occured while setting file metadata!");

        println!("{}", String::from("File metadata set successfully!").bold());
    }
}

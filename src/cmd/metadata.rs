use clap::{Args, Parser, ValueEnum};
use colored::Colorize;

use crate::{
    api::{
        dirtree::get_dirtree,
        fs_files::{
            get_files, set_file_metadata, Filter, FilterCol, FilterOp, GetFilesOpts, SetMetadata,
        },
    },
    config::CONFIG,
    constants::{MIME_TYPES, UNKNOWN_MIME_TYPE},
    shared_types::CliSubCmd,
    utils::dirtree,
};

#[derive(Parser)]
pub struct MetadataCommand {
    /// name of the file
    filename: String,

    #[arg(long)]
    /// the directory path the file is present in (not needed in usual cases, uses the WD set by default)
    dirpath: Option<String>,

    #[arg(long)]
    /// provide a new file type for example: "json", "jpeg", "pdf"
    set_file_type: Option<String>,

    #[arg(long)]
    /// provide a new name to set the file
    set_name: Option<String>,

    #[arg(long)]
    visibility: Option<Visibility>,
}

#[derive(Clone, ValueEnum)]
pub enum Visibility {
    Public,
    Private,
}

impl CliSubCmd for MetadataCommand {
    async fn run(&self) {
        let config = CONFIG.try_lock().unwrap();

        let wd = config.get_wd();

        let filetype = match &self.set_file_type {
            Some(file_type) => {
                Some(*MIME_TYPES.get(file_type).expect("file type not supported please report to add this file type or leave blank to use 'application/octet-stream'"))
            },
            None => None
        };

        let res = get_dirtree(&config)
            .await
            .expect("error occured while fetching dirtree!");

        let subtree = match &self.dirpath {
            Some(dirpath) => {
                let abs_path = dirtree::get_absolute_path(dirpath, wd);
                res.dirtree
                    .get_sub_tree(&abs_path)
                    .expect("incorrect path given, corresponding directory cannot be found!")
            }
            None => res
                .dirtree
                .get_sub_tree(&wd)
                .expect("incorrect working directory set, please change your working directory!"),
        };

        let filters = Some(vec![Filter(
            FilterCol::Name,
            FilterOp::Eq,
            serde_json::json!(&self.filename),
        )]);
        let opts = GetFilesOpts::new(filters);
        let res = get_files(&config, &subtree.id, Some(opts))
            .await
            .expect("error occured while fetching files!");

        let fs_file = res
            .files
            .get(0)
            .expect("error occured, no file found with provided name!");

        let metadata = SetMetadata {
            name: self.set_name.as_deref(),
            storage_id: Some(&fs_file.storage_id),
            file_type: filetype,
            is_public: match self.visibility {
                Some(Visibility::Public) => Some(true),
                Some(Visibility::Private) => Some(false),
                None => None,
            },
        };
        set_file_metadata(&config, metadata)
            .await
            .expect("error occured while setting file metadata!");

        println!("{}", String::from("File metadata set successfully!").bold());
    }
}

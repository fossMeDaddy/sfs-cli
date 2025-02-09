use chrono::Duration;
use clap::Parser;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;

use crate::{
    api::{
        self,
        dirtree::MvOpts,
        fs_files::{
            CliColFilters, Filter, FilterCol, FilterGroup, FilterGroupType, FilterOp, GetFilesOpts,
            Order, OrderCol,
        },
    },
    constants::{self, MIME_TYPES},
    shared_types::{self, AccessToken, AccessTokenPermission, ApiResponse, CliSubCmd, DirTree},
    state::STATE,
    utils::{self, files, x2str},
};

#[derive(Parser)]
pub struct TreeCommand {
    #[arg(short, long)]
    /// level of depth to print the directory tree, defaults to printing full tree
    level: Option<i16>,

    /// start this path with a "/" to indicate absolute path, if in a wd "." & ".." are also
    /// supported
    dirpath: Option<String>,
}

#[derive(Parser)]
pub struct MkdirCommand {
    /// full path to the directory to create, won't throw error if directory already exists
    dirpath: String,
}

#[derive(Parser)]
pub struct RmdirCommand {
    /// full path to the directory to remove, throws error if directory does not exist
    dirpath: String,
}

#[derive(Parser)]
pub struct RmCommand {
    /// names of the files to delete in the selected directory
    filenames: Vec<String>,

    #[arg(long)]
    /// path of the remote dir containing the files to be deleted (defaults to currently set WD)
    dirpath: Option<String>,
}

#[derive(Parser)]
pub struct MvdirCommand {
    /// path of the directory to move
    dirpath: String,

    /// the new location of the directory
    new_dirpath: String,
}

#[derive(Parser)]
pub struct MvCommand {
    /// path of the file to move
    filepath: String,

    /// new location of the file
    new_filepath: String,
}

#[derive(Parser)]
pub struct Cd {
    /// full path to the directory to set as working directory.
    dirpath: String,
}

#[derive(Parser)]
pub struct LsCommand {
    /// specify path to the remote directory to list files of. defaults to currently selected WD
    dirpath: Option<String>,

    #[arg(short, long)]
    /// limit number of files received
    limit: Option<usize>,

    #[arg(short, long)]
    page: Option<usize>,

    #[arg(short, long)]
    /// search by file name. use '%' to search as a pattern. (e.g. "myfilename.json", "v%_myexe.%")
    name: Option<String>,

    #[arg(short, long)]
    /// provide a '.' prefixed file extension name or a MIME type. (e.g. ".json", "image/jpeg")
    type_: Option<String>,

    #[arg(long)]
    /// show only public files
    public: bool,

    #[arg(long)]
    /// show only encrypted files
    encrypted: Option<bool>,

    #[arg(long)]
    /// order by a column
    order_by: Option<OrderCol>,

    #[arg(long)]
    /// order in which given column should be ordered by
    order: Option<Order>,

    #[arg(long)]
    /// show files in trash bin (these files are permanently deleted after ~7 days)
    trash: bool,

    #[command(flatten)]
    filters: CliColFilters,
}

#[derive(Parser)]
pub struct PwdCommand;

#[derive(Parser)]
pub struct UrlCommand {
    /// path to file, can be relative to currently set WD or can be absolute starting with "/"
    path: String,

    #[command(flatten)]
    exp_input: shared_types::CmdExpiryParams,
}

#[derive(Parser)]
pub struct TouchCommand {
    /// path of the new file, can be relative or absolute starting with "/"
    path: String,

    /// mark the file public
    #[arg(long, short)]
    public: bool,

    #[command(flatten)]
    exp_input: Option<shared_types::CmdExpiryParams>,
}

#[derive(Serialize, Debug)]
struct Req<'a> {
    path: &'a str,
}

impl CliSubCmd for TreeCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let res = api::dirtree::get_dirtree()
            .await
            .expect("Unexpected error occured while fetching dirtree!");

        let mut opts = utils::dirtree::PrintDirTreeOpts::get_default_opts();
        opts.file_counts = Some(&res.file_counts);
        opts.level = self.level.unwrap_or(i16::MAX);
        opts.print_note = true;
        opts.cwd_dir_path = wd;

        let dirpath = match &self.dirpath {
            Some(path) => path.as_str(),
            None => wd,
        };
        let abs_path = utils::dirtree::get_absolute_path(dirpath, wd);
        let subtree = match res.dirtree.get_sub_tree(&abs_path) {
            Some(dirtree) => dirtree,
            None => {
                println!(
                    "Invalid path '{}' provided, no path matching the given path exists!",
                    abs_path
                );
                return;
            }
        };

        println!("Directory tree ({}):", abs_path);
        println!("{}", subtree.print_dir_tree(&opts));
    }
}

impl CliSubCmd for MkdirCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let abs_path = utils::dirtree::get_absolute_path(&self.dirpath, wd);

        let dirtree = api::dirtree::mkdir(&abs_path)
            .await
            .expect("error occured while calling 'mkdir' api!");

        let subtree = dirtree.get_sub_tree(wd).unwrap_or(&dirtree);

        let mut print_dirtree_opts = utils::dirtree::PrintDirTreeOpts::get_default_opts();
        print_dirtree_opts.cwd_dir_path = wd;

        println!("Directory tree (/{}):", subtree.name);
        println!("{}", subtree.print_dir_tree(&print_dirtree_opts));
    }
}

impl CliSubCmd for RmdirCommand {
    async fn run(&self) {
        let mut url = api::get_base_url().expect("config issue, cannot fetch base url");
        url.set_path("fs/rmdir");

        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let req = Req {
            path: &utils::dirtree::get_absolute_path(&self.dirpath, wd),
        };
        let res = api::get_builder(reqwest::Method::POST, url)
            .expect("error occured while building request")
            .json(&req)
            .send()
            .await
            .expect("error occured while sending request");

        if !res.status().is_success() {
            println!(
                "{}",
                String::from("Error occured while removing directory!").red()
            );
            println!("{}", res.text().await.unwrap().bright_black());
            return;
        }

        let res_data: ApiResponse<DirTree> = res.json().await.unwrap();
        let dirtree = res_data.data.unwrap();

        let mut print_dirtree_opts = utils::dirtree::PrintDirTreeOpts::get_default_opts();
        print_dirtree_opts.cwd_dir_path = wd;

        println!("Directory tree (/):");
        println!("{}", dirtree.print_dir_tree(&print_dirtree_opts));
    }
}

impl CliSubCmd for MvdirCommand {
    async fn run(&self) {
        let mut url = api::get_base_url().expect("config issue, cannot fetch base url");
        url.set_path("fs/mvdir");

        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Req<'a> {
            path: &'a str,
            new_path: &'a str,
        }
        let req = Req {
            path: &utils::dirtree::get_absolute_path(&self.dirpath, wd),
            new_path: &utils::dirtree::get_absolute_path(&self.new_dirpath, wd),
        };

        let res = api::get_builder(reqwest::Method::POST, url)
            .expect("error occured while building request")
            .json(&req)
            .send()
            .await
            .expect("error occured while sending request");

        let res_data: ApiResponse<DirTree> = res.json().await.unwrap();
        let dirtree = res_data.data.unwrap();

        let mut print_dirtree_opts = utils::dirtree::PrintDirTreeOpts::get_default_opts();
        print_dirtree_opts.cwd_dir_path = wd;

        println!("Directory tree (/):");
        println!("{}", dirtree.print_dir_tree(&print_dirtree_opts));
    }
}

impl CliSubCmd for Cd {
    async fn run(&self) {
        let res = api::dirtree::get_dirtree()
            .await
            .expect("Unexpected error occured while fetching dirtree!");

        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let dirpath = &utils::dirtree::get_absolute_path(&self.dirpath, wd);
        let sub_dirtree = res.dirtree.get_sub_tree(dirpath);
        if sub_dirtree.is_none() {
            println!("{}", String::from("Path does not exists!").red());
            return;
        }

        drop(state);

        let mut state_mut = STATE.write().unwrap();
        state_mut
            .set_wd(&dirpath)
            .expect("Error occured while setting a working directory!");
    }
}

impl CliSubCmd for PwdCommand {
    async fn run(&self) {
        let state = STATE.write().expect(
            "failed to acquire lock over config, THIS SHOULD NOT HAPPEN, PLEASE REPORT BUG!!",
        );

        println!("{}", state.get_wd());
    }
}

impl LsCommand {
    fn get_file_cache_duration_str(secs: u64) -> String {
        match Duration::new(secs as i64, 0) {
            Some(d) => {
                format!("CacheTTL={}", x2str::duration2str(d))
            }
            None => String::from("INVALID_DURATION"),
        }
    }
}

impl CliSubCmd for LsCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let dirpath = match &self.dirpath {
            Some(dirpath) => utils::dirtree::get_absolute_path(dirpath, wd),
            None => wd.to_string(),
        };

        let mut filters: Vec<FilterGroup> = vec![];
        let mut main_and_group = FilterGroup {
            type_: FilterGroupType::And,
            filters: vec![],
        };
        main_and_group
            .filters
            .extend(self.filters.parse_get_filters().expect("invalid filters!"));
        if self.trash {
            main_and_group.filters.push(Filter(
                FilterCol::DeletedAt,
                FilterOp::Ne,
                serde_json::json!(null),
            ));
        }
        if self.public {
            main_and_group.filters.push(Filter(
                FilterCol::IsPublic,
                FilterOp::Eq,
                serde_json::json!(self.public),
            ));
        }
        if let Some(encrypted) = self.encrypted {
            main_and_group.filters.push(Filter(
                FilterCol::Encryption,
                match encrypted {
                    true => FilterOp::IsNotNull,
                    false => FilterOp::IsNull,
                },
                serde_json::json!(null),
            ));
        }
        if let Some(ref name) = self.name {
            main_and_group.filters.push(Filter(
                FilterCol::Name,
                match name.contains("%") {
                    true => FilterOp::Like,
                    false => FilterOp::Eq,
                },
                serde_json::json!(name),
            ));
        }
        if let Some(ref filetype) = self.type_ {
            let mut group = FilterGroup {
                type_: FilterGroupType::Or,
                filters: vec![],
            };

            if filetype.starts_with(".") {
                let file_ext = filetype;

                let like_str = format!("%{file_ext}");

                group.filters.push(Filter(
                    FilterCol::Name,
                    FilterOp::Like,
                    serde_json::json!(like_str),
                ));

                if let Some(mime_type) = MIME_TYPES.get(&file_ext[1..]) {
                    group.filters.push(Filter(
                        FilterCol::ContentType,
                        FilterOp::Eq,
                        serde_json::json!(mime_type),
                    ));
                }
            } else {
                let content_type = filetype;

                group.filters.push(Filter(
                    FilterCol::ContentType,
                    FilterOp::Eq,
                    serde_json::json!(filetype),
                ));

                if let Some((file_ext, _)) =
                    MIME_TYPES.into_iter().find(|(_, v)| *v == content_type)
                {
                    let like_str = format!("%.{file_ext}");
                    group.filters.push(Filter(
                        FilterCol::Name,
                        FilterOp::Like,
                        serde_json::json!(like_str),
                    ));
                }
            }

            filters.push(group);
        }
        filters.push(main_and_group);

        let get_file_opts = GetFilesOpts {
            dir_path: dirpath.clone(),
            filters: Some(filters),
            limit: self.limit,
            page: self.page,
            order_by: self.order_by,
            order: self.order,
        };
        let res = api::fs_files::get_files(Some(get_file_opts))
            .await
            .expect("error occured while fetching fetching files");

        if res.files.len() == 0 {
            println!("{}", "no results found.".to_string().bold());
            return;
        }

        let mut pretty_file_sizes: Vec<String> = vec![];
        let mut file_type_padding = 0;
        let mut file_size_padding = 0;
        let mut file_cache_age_padding = 0;
        res.files.iter().for_each(|f| {
            let pretty_file_size = utils::x2str::bytes2str(f.file_size as u64);

            file_type_padding = file_type_padding.max(f.get_filetype().len());
            file_size_padding = file_size_padding.max(pretty_file_size.len());
            file_cache_age_padding = file_cache_age_padding
                .max(Self::get_file_cache_duration_str(f.cache_max_age_seconds).len());

            pretty_file_sizes.push(pretty_file_size);
        });

        for (file, pretty_file_size) in res.files.iter().zip(pretty_file_sizes.iter()) {
            let mut emo_tags = String::new();
            emo_tags += match file.encryption.is_some() {
                true => "🔒",
                false => "",
            };
            emo_tags += match file.is_public {
                true => "🌐",
                false => "",
            };

            let cache_ttl_str =
                Self::get_file_cache_duration_str(file.cache_max_age_seconds).dimmed();
            print!(
                "{0:<1$} ",
                if file.cache_max_age_seconds == 0 {
                    cache_ttl_str.red()
                } else {
                    cache_ttl_str
                },
                file_cache_age_padding
            );
            print!(
                "{} ",
                file.updated_at
                    .format(constants::LOCAL_DATETIME_FORMAT)
                    .to_string()
                    .dimmed()
                    .magenta()
            );
            print!("{0:>1$} ", pretty_file_size.bold(), file_size_padding);
            print!("{0:>1$} ", file.get_filetype(), file_type_padding);
            print!("{} ", file.name.bold().cyan());
            print!("{} ", emo_tags);
            println!();
        }

        let page = self.page.unwrap_or(1);
        let offset = (page - 1) * res.page_size;

        println!();
        println!(
            "{} {} {}",
            format!("(page: {})", page),
            format!(
                "showing {}-{} of {} in",
                offset + 1,
                offset + res.files.len(),
                res.count
            )
            .dimmed(),
            dirpath.bold()
        );
    }
}

impl CliSubCmd for RmCommand {
    async fn run(&self) {
        if self.filenames.len() == 0 {
            println!("{}", String::from("no file names provided.").red());
        }

        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let res = api::dirtree::get_dirtree()
            .await
            .expect("error occured while fetching dirtree!");

        let dirpath = match &self.dirpath {
            Some(dirpath) => {
                let abs_path = utils::dirtree::get_absolute_path(dirpath, wd);
                res.dirtree
                    .get_sub_tree(&abs_path)
                    .expect("provided dirpath does not exist!");

                dirpath
            }
            None => {
                res.dirtree
                    .get_sub_tree(wd)
                    .expect("current WD incorrectly set, please switch to a valid WD!");

                wd
            }
        };

        let deleted_files = api::fs_files::delete_files(&api::fs_files::DeleteFilesReqBody {
            dir_path: dirpath,
            file_names: &self.filenames,
        })
        .await
        .expect("error occured while deleteing files!");

        println!(
            "{}",
            format!("Deleted {} files successfully.", deleted_files.len()).bold()
        )
    }
}

impl CliSubCmd for UrlCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let abs_path = utils::dirtree::get_absolute_path(&self.path, wd);
        let (dirpath, filename) = utils::dirtree::split_path(&abs_path);

        let filters = FilterGroup {
            type_: FilterGroupType::And,
            filters: vec![Filter(FilterCol::Name, FilterOp::Eq, json!(filename))],
        };
        let mut opts = GetFilesOpts::new(dirpath.to_string());
        opts.filters = Some(vec![filters]);

        let res_files = api::fs_files::get_files(Some(opts))
            .await
            .expect("error fetching file!");

        let file = res_files.files.first().expect("no file found!");

        let access_token: Option<String> = match file.is_public {
            true => None,
            false => {
                let perms: AccessTokenPermission = "r"
                    .parse()
                    .expect("error occured while generating permissions for access token!");
                let acpl = vec![utils::tokens::get_acp(perms, &abs_path)];

                let res_data =
                    api::tokens::generate_access_token(&acpl, &self.exp_input.get_expires_at())
                        .await
                        .expect("error occured while generating access token!");

                Some(res_data.access_token)
            }
        };

        let share_url =
            files::get_share_url(access_token.as_deref(), &file.storage_id).expect(&format!(
                "error generating shareable url for specified path '{}'",
                abs_path
            ));
        println!("{}", share_url.to_string().bold().cyan());

        if let Some(token) = access_token {
            let access_token: AccessToken = token.parse().expect("generate access token seems invalid! this SHOULD NOT HAPPEN!!! please report this bug.");

            println!();
            println!(
                "{}",
                format!(
                    "expires_at: {}",
                    access_token
                        .expires_at
                        .format(constants::LOCAL_DATETIME_FORMAT)
                        .to_string()
                        .magenta()
                )
                .dimmed()
            );
            println!(
                "{}",
                format!("acpl: {}", access_token.acpl.join(", ").blue()).dimmed()
            );
        }
    }
}

impl CliSubCmd for MvCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let _ = api::dirtree::mv(&MvOpts {
            file_path: &utils::dirtree::get_absolute_path(&self.filepath, wd),
            new_file_path: &utils::dirtree::get_absolute_path(&self.new_filepath, wd),
        })
        .await
        .expect(&format!(
            "error occured while moving file from '{}' to '{}'",
            self.filepath, self.new_filepath
        ));

        println!("new file location: {}", self.new_filepath.bold());
    }
}

impl CliSubCmd for TouchCommand {
    async fn run(&self) {
        let state = &STATE.read().unwrap();
        let wd = state.get_wd();

        let abs_path = utils::dirtree::get_absolute_path(&self.path, wd);
        let (dirpath, filename) = utils::dirtree::split_path(&abs_path);

        if let Some(_) = api::fs_files::get_file(&abs_path)
            .await
            .expect("error occured while connecting to API!")
        {
            return;
        };

        let empty_stream = async_stream::try_stream! {
            yield Vec::new();
        };
        let empty_stream = Box::pin(empty_stream);

        let upload_metadata = shared_types::UploadBlobMetadata {
            name: filename.to_string(),
            content_type: None,
            dir_path: dirpath.to_string(),
            force_write: false,
            is_public: self.public,
            deleted_at: self.exp_input.as_ref().map(|exp| exp.get_expires_at()),
            encryption: None,
            cache_max_age_seconds: None,
        };

        api::uploads::upload_blob_stream(empty_stream, &upload_metadata)
            .await
            .expect(&format!(
                "error occured while creating file at path '{}'",
                self.path
            ));
    }
}

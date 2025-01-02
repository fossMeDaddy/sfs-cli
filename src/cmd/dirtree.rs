use clap::Parser;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;

use crate::{
    api::{
        self,
        dirtree::MvOpts,
        fs_files::{CliColFilters, Filter, FilterCol, FilterOp, GetFilesOpts, Order, OrderCol},
    },
    config::CONFIG,
    constants,
    shared_types::{
        AccessToken, AccessTokenPermission, ApiResponse, AppContext, CliSubCmd, DirTree,
    },
    state::STATE,
    utils::{self, files},
};

use super::tokens::ExpInput;

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
    dirpath: Option<String>,
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
    /// explicitly type the name of a file to get it's info
    name: Option<String>,

    #[arg(short, long)]
    /// filter by file type (e.g. "json", "jpeg", "pdf", "mp4")
    type_: Option<String>,

    #[arg(long)]
    /// show only public files
    public: bool,

    #[arg(long)]
    /// show only encrypted files
    encrypted: bool,

    #[arg(long)]
    /// order by given column, defaults to creation date in descending order
    order_by: Option<OrderCol>,

    #[arg(long)]
    /// order in which given column should be ordered by
    order: Option<Order>,

    #[arg(long)]
    /// show files in trash bin (these files are permanently deleted after ~7 days)
    trash: bool,

    #[command(flatten)]
    /// e.g. `--deleted-at ">"`
    filters: CliColFilters,
}

#[derive(Parser)]
pub struct PwdCommand;

#[derive(Parser)]
pub struct UrlCommand {
    /// path to file, can eb relative to currently set WD or can be absolute starting with "/"
    path: String,

    #[command(flatten)]
    exp_input: ExpInput,
}

#[derive(Serialize, Debug)]
struct Req<'a> {
    path: &'a str,
}

impl CliSubCmd for TreeCommand {
    async fn run(&self) {
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };
        let wd = ctx.config.get_wd();

        let res = api::dirtree::get_dirtree(&ctx)
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
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };

        let mut url = api::get_base_url(&ctx).expect("invalid url!");
        url.set_path("fs/mkdir");

        let wd = ctx.config.get_wd();

        let dirpath = match &self.dirpath {
            Some(dirpath) => &utils::dirtree::get_absolute_path(&dirpath, wd),
            None => wd,
        };

        let req = Req { path: dirpath };

        let res = api::get_builder(reqwest::Method::POST, url)
            .expect("error occured while building request")
            .json(&req)
            .send()
            .await
            .expect("error occured while sending request");

        if !res.status().is_success() {
            println!(
                "{}",
                String::from("Error occured while creating directory!").red()
            );
            println!("{}", res.text().await.unwrap().bright_red());
            return;
        }

        let res_data: ApiResponse<DirTree> = res.json().await.unwrap();
        let dirtree = res_data.data.unwrap();
        let subtree = dirtree.get_sub_tree(wd).unwrap_or(&dirtree);

        let mut print_dirtree_opts = utils::dirtree::PrintDirTreeOpts::get_default_opts();
        print_dirtree_opts.cwd_dir_path = wd;

        println!("Directory tree (/{}):", subtree.name);
        println!("{}", subtree.print_dir_tree(&print_dirtree_opts));
    }
}

impl CliSubCmd for RmdirCommand {
    async fn run(&self) {
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };

        let mut url = api::get_base_url(&ctx).expect("config issue, cannot fetch base url");
        url.set_path("fs/rmdir");

        let wd = ctx.config.get_wd();

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
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };

        let mut url = api::get_base_url(&ctx).expect("config issue, cannot fetch base url");
        url.set_path("fs/mvdir");

        let wd = ctx.config.get_wd();

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
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };
        let res = api::dirtree::get_dirtree(&ctx)
            .await
            .expect("Unexpected error occured while fetching dirtree!");

        let wd = ctx.config.get_wd();

        let dirpath = &utils::dirtree::get_absolute_path(&self.dirpath, wd);
        let sub_dirtree = res.dirtree.get_sub_tree(dirpath);
        if sub_dirtree.is_none() {
            println!("{}", String::from("Path does not exists!").red());
            return;
        }

        drop(ctx);

        let mut config_mut = CONFIG.try_lock().unwrap();
        config_mut
            .set_wd(&dirpath)
            .expect("Error occured while setting a working directory!");
    }
}

impl CliSubCmd for PwdCommand {
    async fn run(&self) {
        let config = CONFIG.try_lock().expect(
            "failed to acquire lock over config, THIS SHOULD NOT HAPPEN, PLEASE REPORT BUG!!",
        );

        println!("{}", config.get_wd());
    }
}

impl CliSubCmd for LsCommand {
    async fn run(&self) {
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };
        let wd = ctx.config.get_wd();

        let page = self.page.unwrap_or(1);

        let mut filters: Vec<Filter> = vec![];
        filters.extend(self.filters.parse_get_filters().expect("invalid filters!"));

        let dirpath = match &self.dirpath {
            Some(dirpath) => utils::dirtree::get_absolute_path(dirpath, wd),
            None => wd.to_string(),
        };

        if self.trash {
            filters.push(Filter(
                FilterCol::DeletedAt,
                FilterOp::Ne,
                serde_json::json!(null),
            ));
        }
        if self.public {
            filters.push(Filter(
                FilterCol::IsPublic,
                FilterOp::Eq,
                serde_json::json!(self.public),
            ));
        }
        if self.encrypted {
            filters.push(Filter(
                FilterCol::IsEncrypted,
                FilterOp::Eq,
                serde_json::json!(self.encrypted),
            ));
        }
        if let Some(ref name) = self.name {
            filters.push(Filter(
                FilterCol::Name,
                FilterOp::Eq,
                serde_json::json!(name),
            ));
        }
        if let Some(ref filetype) = self.type_ {
            let filetype = match constants::MIME_TYPES.get(filetype.trim_matches('.')) {
                Some(filetype) => filetype,
                None => constants::UNKNOWN_MIME_TYPE,
            };

            filters.push(Filter(
                FilterCol::FileType,
                FilterOp::Eq,
                serde_json::json!(filetype),
            ));
        }

        let get_file_opts = GetFilesOpts {
            dir_path: &dirpath,
            filters: Some(&filters),
            limit: self.limit,
            page: self.page,
            order_by: self.order_by,
            order: self.order,
        };
        let res = api::fs_files::get_files(&ctx, Some(get_file_opts))
            .await
            .expect("error occured while fetching fetching files");

        if res.files.len() == 0 {
            println!("{}", "no results found.".to_string().bold());
            return;
        }

        let mut pretty_file_sizes: Vec<String> = vec![];
        let mut file_type_padding = 0;
        let mut file_size_padding = 0;
        res.files.iter().for_each(|f| {
            let pretty_file_size = utils::files::get_pretty_size(f.file_size);

            file_type_padding = file_type_padding.max(f.get_filetype().len());
            file_size_padding = file_size_padding.max(pretty_file_size.len());

            pretty_file_sizes.push(pretty_file_size);
        });

        for (file, pretty_file_size) in res.files.iter().zip(pretty_file_sizes.iter()) {
            let mut emo_tags = String::new();
            emo_tags += match file.is_encrypted {
                true => "🔒",
                false => "",
            };
            emo_tags += match file.is_public {
                true => "🌐",
                false => "",
            };

            print!(
                "{} ",
                file.created_at
                    .format(constants::LOCAL_DATETIME_FORMAT)
                    .to_string()
                    .dimmed()
                    .magenta()
            );
            print!("{0:>1$} ", pretty_file_size.bold(), file_size_padding);
            print!("{0:>1$} ", file.get_filetype().dimmed(), file_type_padding);
            print!("{} ", file.name.bold().cyan());
            print!("{} ", emo_tags);
            println!();
        }

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

        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };
        let wd = ctx.config.get_wd();

        let res = api::dirtree::get_dirtree(&ctx)
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

        let deleted_files = api::fs_files::delete_files(
            &ctx,
            &api::fs_files::DeleteFilesReqBody {
                dir_path: dirpath,
                file_names: &self.filenames,
            },
        )
        .await
        .expect("error occured while deleteing files!");

        println!(
            "{}",
            format!("Deleted {} files successfully!", deleted_files.len()).bold()
        )
    }
}

impl CliSubCmd for UrlCommand {
    async fn run(&self) {
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };
        let wd = ctx.config.get_wd();

        let abs_path = utils::dirtree::get_absolute_path(&self.path, wd);
        let (dirpath, filename) = utils::dirtree::split_path(&abs_path);

        let filters = vec![Filter(FilterCol::Name, FilterOp::Eq, json!(filename))];
        let opts = GetFilesOpts::new(dirpath, Some(&filters));
        let res_files = api::fs_files::get_files(&ctx, Some(opts))
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

                let res_data = api::tokens::generate_access_token(
                    &ctx,
                    &acpl,
                    &self.exp_input.get_expires_at(),
                )
                .await
                .expect("error occured while generating access token!");

                Some(res_data.access_token)
            }
        };

        let share_url = files::get_share_url(access_token.as_deref(), &file.storage_id, &ctx)
            .expect(&format!(
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
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };
        let wd = ctx.config.get_wd();

        let _ = api::dirtree::mv(
            &ctx,
            &MvOpts {
                file_path: &utils::dirtree::get_absolute_path(&self.filepath, wd),
                new_file_path: &utils::dirtree::get_absolute_path(&self.new_filepath, wd),
            },
        )
        .await
        .expect(&format!(
            "error occured while moving file from '{}' to '{}'",
            self.filepath, self.new_filepath
        ));

        println!("new file location: {}", self.new_filepath.bold());
    }
}

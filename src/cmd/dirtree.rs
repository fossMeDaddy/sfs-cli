use clap::Parser;
use colored::Colorize;
use serde::Serialize;

use crate::{
    api::{
        self,
        fs_files::{CliColFilters, Filter, FilterCol, FilterOp, GetFilesOpts, Order, OrderCol},
    },
    config::CONFIG,
    constants,
    shared_types::{ApiResponse, CliSubCmd, DirTree},
    utils,
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
    dirpath: Option<String>,
}

#[derive(Parser)]
pub struct RmdirCommand {
    /// full path to the directory to remove, throws error if directory does not exist
    dirpath: String,
}

#[derive(Parser)]
pub struct MvdirCommand {
    /// full path to the directory to move
    dirpath: String,

    /// full path to the new location of the directory
    new_dirpath: String,
}

#[derive(Parser)]
pub struct SetwdCommand {
    /// full path to the directory to set as working directory.
    dirpath: String,
}

#[derive(Parser)]
pub struct LsCommand {
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
pub struct RmCommand {
    /// names of the files to delete in the selected directory
    filenames: Vec<String>,

    #[arg(long)]
    /// path of the remote dir containing the files to be deleted (defaults to currently set WD)
    dirpath: Option<String>,
}

#[derive(Serialize, Debug)]
struct Req<'a> {
    path: &'a str,
}

impl CliSubCmd for TreeCommand {
    async fn run(&self) {
        let config = CONFIG.try_lock().unwrap();
        let wd = config.get_wd();

        let res = api::dirtree::get_dirtree(&config)
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
                println!("Invalid path provided, no path matching the given path exists!");
                return;
            }
        };

        println!("Directory tree ({}):", abs_path);
        println!("{}", subtree.print_dir_tree(&opts));
    }
}

impl CliSubCmd for MkdirCommand {
    async fn run(&self) {
        let config = CONFIG.try_lock().unwrap();

        let mut url = api::get_base_url(&config).expect("invalid url!");
        url.set_path("fs/mkdir");

        let wd = config.get_wd();

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
        let config = CONFIG.try_lock().unwrap();

        let mut url = api::get_base_url(&config).expect("config issue, cannot fetch base url");
        url.set_path("fs/rmdir");

        let wd = config.get_wd();

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
        let config = CONFIG.try_lock().unwrap();

        let mut url = api::get_base_url(&config).expect("config issue, cannot fetch base url");
        url.set_path("fs/mvdir");

        let wd = config.get_wd();

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

impl CliSubCmd for SetwdCommand {
    async fn run(&self) {
        let mut config_mut = CONFIG.try_lock().unwrap();

        let res = api::dirtree::get_dirtree(&config_mut)
            .await
            .expect("Unexpected error occured while fetching dirtree!");

        let wd = config_mut.get_wd();

        let dirpath = &utils::dirtree::get_absolute_path(&self.dirpath, wd);
        let sub_dirtree = res.dirtree.get_sub_tree(dirpath);
        if sub_dirtree.is_none() {
            println!("{}", String::from("Path does not exists!").red());
            return;
        }

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
        let config = CONFIG.try_lock().unwrap();
        let wd = config.get_wd();

        let page = self.page.unwrap_or(1);

        let res = api::dirtree::get_dirtree(&config)
            .await
            .expect("error occured while fetching dirtree");

        let subtree = res.dirtree.get_sub_tree(wd).expect(
            "cannot fetch correct dirtree, are you sure you're in a valid working directory?",
        );

        let mut filters: Vec<Filter> = vec![];
        filters.extend(self.filters.parse_get_filters().expect("invalid filters!"));

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
            filters: Some(filters),
            limit: self.limit,
            page: self.page,
            order_by: self.order_by,
            order: self.order,
        };
        let res = api::fs_files::get_files(&config, &subtree.id, Some(get_file_opts))
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
            let pretty_file_size = f.get_pretty_size();

            file_type_padding = file_type_padding.max(f.file_type.to_string().len());
            file_size_padding = file_size_padding.max(pretty_file_size.len());

            pretty_file_sizes.push(pretty_file_size);
        });
        println!("{} {}", file_size_padding, file_type_padding);

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
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .dimmed()
                    .magenta()
            );
            print!("{0:>1$} ", pretty_file_size.bold(), file_size_padding);
            print!("{0:>1$} ", file.file_type.dimmed(), file_type_padding);
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
            wd.bold()
        );
    }
}

impl CliSubCmd for RmCommand {
    async fn run(&self) {
        if self.filenames.len() == 0 {
            println!("{}", String::from("no file names provided.").red());
        }

        let config = CONFIG.try_lock().unwrap();

        let wd = config.get_wd();

        let res = api::dirtree::get_dirtree(&config)
            .await
            .expect("error occured while fetching dirtree!");
        let subtree = match &self.dirpath {
            Some(dirpath) => {
                let abs_path = utils::dirtree::get_absolute_path(dirpath, wd);
                res.dirtree
                    .get_sub_tree(&abs_path)
                    .expect("provided dirpath does not exist!")
            }
            None => res
                .dirtree
                .get_sub_tree(wd)
                .expect("current WD incorrectly set, please switch to a valid WD!"),
        };

        for filename in &self.filenames {
            // spawn async task to delete the file
        }
        // log the errors
        // if everything ok, print success
    }
}

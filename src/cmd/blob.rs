use std::{
    collections::HashSet,
    env::{current_dir, var},
    path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
    rc::Rc,
    str::FromStr,
    sync::Arc,
};

use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use clap::{Args, Parser};
use colored::*;
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use inquire::Confirm;
use orion::{aead::streaming, kdf};
use serde_json::json;
use tokio::{
    fs,
    io::{self, AsyncWriteExt},
    sync::mpsc,
    task,
};
use url::Url;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    api::{self, fs_files::*, uploads::UploadFileOpts},
    constants,
    shared_types::{
        self, AccessTokenPermission, CliSubCmd, FsFile, PermissionChar, UploadBlobMetadata,
    },
    state::STATE,
    utils::{
        self,
        crypto::derive_key_from_password,
        dirtree,
        files::{self, get_share_url},
        paths, str2x, tokens,
    },
};

#[derive(Args)]
#[group(multiple = true)]
pub struct CmdUploadParams {
    #[arg(long)]
    /// mark the uploaded file(s) public. public files do not require access tokens to be viewed
    public: bool,

    #[arg(long)]
    /// if this flag is true, the program will try to read from shell variable "PASSWORD" or prompt for a password
    password: bool,

    #[arg(long)]
    /// MIME type for the content being uploaded. (guesses from file extension if unspecified, no effect for multi-file upload)
    content_type: Option<String>,

    #[arg(long)]
    /// generate a safely-shareable url containing read-only permissions for the uploaded file. (default expiry: 30 mins)
    share: bool,

    #[arg(long, value_parser = str2x::str2duration)]
    /// provide custom expiry time for the share url. (e.g "AAdBBhCCmDDs", "7d30s", "2h30m12s", "30m" [default])
    share_exp: Option<Duration>,

    #[arg(long, value_parser = str2x::str2duration)]
    /// set the 'max-age' value for CDN cache distribution, defaults to 0 (format: 12d23h34m45s)
    cache_max_age: Option<Duration>,

    #[command(flatten)]
    exp_input: shared_types::CmdExpiryParams,
}
impl CmdUploadParams {
    pub fn into_upload_metadata(
        &self,
        name: String,
        dir_path: String,
        force_write: bool,
        encryption: Option<shared_types::EncryptionMetadata>,
    ) -> shared_types::UploadBlobMetadata {
        shared_types::UploadBlobMetadata {
            name,
            dir_path,
            is_public: self.public,
            force_write,
            deleted_at: self.get_deleted_at(),
            cache_max_age_seconds: self.get_cache_max_age_seconds(),
            content_type: self.content_type.clone(),
            encryption,
        }
    }

    pub fn get_password(&self) -> Option<String> {
        match var("PASSWORD") {
            Ok(pwd) => Some(pwd),
            Err(_) => match self.password {
                true => Some(
                    dialoguer::Password::new()
                        .with_prompt("create password (remember this password!)")
                        .with_confirmation("confirm", "passwords don't match!")
                        .interact()
                        .unwrap(),
                ),
                false => None,
            },
        }
    }

    pub fn is_share(&self) -> bool {
        self.share || self.share_exp.is_some()
    }

    // get default now+30mins expiry date or the custom one in `share_exp`
    pub fn get_share_expiry(&self) -> DateTime<Utc> {
        self.share_exp
            .map(|s| Utc::now() + s)
            .unwrap_or(Utc::now() + Duration::minutes(30))
    }

    pub fn get_deleted_at(&self) -> Option<DateTime<Utc>> {
        match self.exp_input.is_unset() {
            true => None,
            false => Some(self.exp_input.get_expires_at()),
        }
    }

    pub fn get_cache_max_age_seconds(&self) -> Option<u64> {
        self.cache_max_age.map(|d| d.abs().num_seconds() as u64)
    }
}

#[derive(Parser)]
pub struct UploadBlobCommand {
    /// list of path patterns. (default: files are uploaded in a single zip file in case of multiple files covered by the pattern)
    source_path_patterns: Vec<String>,

    #[arg(long)]
    /// relative or absolute path to an existing remote directory. (default: currently set WD)
    dirpath: Option<String>,

    #[arg(long, short)]
    /// name to save the file as after uploading, a random name is generated if not provided
    name: Option<String>,

    #[arg(long, short)]
    /// update file in case a remote file already exists with same upload path
    force: bool,

    #[command(flatten)]
    upload_params: CmdUploadParams,

    #[arg(long)]
    /// all the files and the directory structures covered under the path pattern will be recursively created in remote dirpath
    recursive: bool,

    #[arg(long)]
    /// do not display any confirm prompts
    no_confirm: bool,
}

#[derive(Parser)]
pub struct SelectCommand {
    filepath: String,

    #[command(flatten)]
    upload_params: CmdUploadParams,
}

#[derive(Parser)]
pub struct CatCommand {
    /// takes in relative path, absolute path, or a url of the remote file
    location_hint: String,
}

impl CliSubCmd for UploadBlobCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let paths: Vec<(u64, PathBuf)> = self
            .source_path_patterns
            .iter()
            .flat_map(|patt| {
                paths::get_paths_from_pattern(patt)
                    .expect("invalid pattern received, could not be resolved to a path!")
            })
            .collect();
        if paths.len() == 0 {
            println!(
                "{}",
                String::from(
                    "No matching files or directories exist! please check your provided patterns."
                )
                .red()
            );
            return;
        }
        let only_paths = paths.iter().map(|(_, p)| p);

        println!(
            "{}",
            format!("files covered by pattern: {}", paths.len()).bold()
        );

        let upload_dirpath = match &self.dirpath {
            Some(dirpath) => dirtree::get_absolute_path(dirpath, wd),
            None => wd.to_string(),
        }
        .to_string();

        let (ref_wd, pretty_paths) =
            paths::get_pretty_paths(paths.iter().map(|(size, p)| (*size, p)));

        println!();
        if ref_wd.len() > 0 {
            println!("{}", ref_wd.bold().dimmed());
        }
        println!("{}", pretty_paths.dimmed().blue());
        println!();

        if !self.no_confirm {
            let confirm = Confirm::new(&format!(
                "{} files will be uploaded at {}, confirm:",
                paths.len(),
                upload_dirpath.bold()
            ))
            .with_default(false)
            .prompt()
            .expect("error occured while displaying confirm prompt!");

            if !confirm {
                println!("Aborted upload.");
                return;
            }
        }

        let password = self.upload_params.get_password();

        if self.recursive {
            let ref_wd = Self::get_ref_wd_from_paths(only_paths.clone());

            let spinner = ProgressBar::new_spinner().with_message("creating directory tree");
            spinner.enable_steady_tick(Duration::milliseconds(50).to_std().unwrap());
            Self::create_dirtree_from_filepaths(wd, ref_wd, only_paths.clone())
                .await
                .unwrap();
            spinner.finish_and_clear();

            self.upload_files(wd, ref_wd, only_paths.clone())
                .await
                .expect("files upload unsuccessful!");

            println!("\n{}", "files uploaded successfully.".bold());
            return;
        }

        let is_zip_file = paths.len() > 1;

        let upload_filepath = if is_zip_file {
            &self.get_zipfile_from_paths(only_paths, password.as_deref())
        } else {
            &paths[0].1
        };
        let filename = self.name.as_deref().unwrap_or(
            upload_filepath
                .file_name()
                .expect(&format!(
                    "invalid path provided '{}'",
                    upload_filepath.to_string_lossy()
                ))
                .to_str()
                .expect("invalid upload filepath! non-utf8 string provided."),
        );
        let file_len = fs::metadata(upload_filepath)
            .await
            .expect("error occured while reading file! metadata could not be read.")
            .len();

        let mut opts = UploadFileOpts::new(
            upload_filepath.clone(),
            password,
            ProgressBar::new(file_len)
                .with_style(utils::misc::get_sized_throughput_progress_style(None)),
        );
        opts.is_zip_file = is_zip_file;
        let fs_file = api::uploads::upload_file(
            self.upload_params.into_upload_metadata(
                filename.to_string(),
                upload_dirpath.clone(),
                self.force,
                None,
            ),
            opts,
        )
        .await
        .expect("error occured while uploading file!");

        if self.upload_params.is_share() {
            let share_url: String;

            if self.upload_params.public {
                share_url = files::get_share_url(None, &fs_file.storage_id)
                    .expect("error occured while generating share url!")
                    .to_string();
            } else {
                let acpl: Vec<String> = vec![tokens::get_acp(
                    AccessTokenPermission::from_str(&PermissionChar::Read.to_string())
                        .expect("Error occured while parsing read access token permission"),
                    &format!("{}/{}", upload_dirpath, fs_file.name),
                )];
                let expires_at = self.upload_params.get_share_expiry();

                let res_data = api::tokens::generate_access_token(&acpl, &expires_at)
                    .await
                    .expect("error occured while generating access token!");

                share_url = files::get_share_url(Some(&res_data.access_token), &fs_file.storage_id)
                    .expect("error occured while generating share url!")
                    .to_string();
            }

            println!("\n{}\n", share_url.blue());
        }

        println!(
            "{}",
            format!(
                "Upload full path: {}",
                dirtree::join_paths(&[&upload_dirpath, &fs_file.name]),
            )
            .dimmed()
        );
    }
}

impl UploadBlobCommand {
    async fn create_dirtree_from_filepaths<'a, I>(
        wd: &'a str,
        ref_wd: &'a str,
        filepaths: I,
    ) -> anyhow::Result<()>
    where
        I: ExactSizeIterator<Item = &'a PathBuf> + Clone,
    {
        let filepaths = filepaths.into_iter();

        let mut err_msg = String::new();

        let mkdir_paths = filepaths
            .map(|filepath| {
                let dirpath = filepath.parent().unwrap_or(Path::new(""));
                let path = dirpath.to_str().unwrap().trim_start_matches(ref_wd);
                utils::dirtree::join_paths(&[wd, path])
            })
            .collect::<HashSet<String>>();
        for res in futures_util::future::join_all(
            mkdir_paths
                .iter()
                .map(|abs_path| api::dirtree::mkdir(&abs_path)),
        )
        .await
        {
            match res {
                Err(err) => err_msg += &format!("{err}\n"),
                Ok(_) => {}
            }
        }

        if err_msg.len() > 0 {
            return Err(anyhow!(err_msg));
        }

        Ok(())
    }

    /// filepaths MUST BE trimmed from ref_wd by the caller
    async fn upload_files<'a, I>(
        &self,
        wd: &'a str,
        ref_wd: &'a str,
        filepaths: I,
    ) -> anyhow::Result<()>
    where
        I: ExactSizeIterator<Item = &'a PathBuf> + Clone,
    {
        let filepaths = filepaths.into_iter();
        let ref_wd = Arc::new(ref_wd.to_string());
        let wd = Arc::new(wd.to_string());
        let multi_progress_bar = Arc::new(MultiProgress::new());

        let progress_padded_labels = filepaths.clone().map(|filepath| {
            filepath
                .to_string_lossy()
                .trim_start_matches(&*ref_wd.clone())
                .trim_start_matches(match ref_wd.len() {
                    0 => "",
                    _ => "/",
                })
                .to_string()
        });
        let padding = progress_padded_labels
            .clone()
            .fold(0, |acc, elem| acc.max(elem.len()));
        let progress_padded_labels =
            progress_padded_labels.map(|label| format!("{0:<1$}", label, padding));

        let mut upload_handles = vec![];
        for (filepath, progress_padded_label) in filepaths.clone().zip(progress_padded_labels) {
            let filepath = filepath.clone();
            let ref_wd = ref_wd.clone();
            let wd = wd.clone();
            let pwd = self.upload_params.get_password();
            let force_write = self.force;
            let multi_progress_bar = multi_progress_bar.clone();

            let handle = task::spawn(async move {
                let mut path_segs: Vec<&str> = match filepath.to_str() {
                    Some(p) => p.trim_start_matches(ref_wd.as_ref()),
                    None => {
                        return Err(anyhow!(
                            "WARNING: path '{}' is not a valid utf8 string! skipping...",
                            filepath.to_string_lossy()
                        ));
                    }
                }
                .split(MAIN_SEPARATOR_STR)
                .collect();

                let filename = match path_segs.pop() {
                    Some(name) => name,
                    None => {
                        return Err(anyhow!(
                            "path '{}' does not contain a filename!",
                            filepath.to_string_lossy()
                        ));
                    }
                };
                let dirpath: String = utils::dirtree::join_paths(&[&wd, &path_segs.join("/")]);

                let file_len = fs::metadata(&filepath).await?.len();

                let progress_bar = ProgressBar::new(file_len)
                    .with_message(filepath.to_string_lossy().to_string())
                    .with_style(utils::misc::get_sized_throughput_progress_style(Some(
                        &progress_padded_label,
                    )));
                multi_progress_bar.add(progress_bar.clone());
                Ok(api::uploads::upload_file(
                    UploadBlobMetadata {
                        name: filename.to_string(),
                        content_type: None,
                        dir_path: dirpath,
                        is_public: false,
                        force_write,
                        encryption: None,
                        cache_max_age_seconds: Some(0),
                        deleted_at: None,
                    },
                    UploadFileOpts::new(filepath.clone(), pwd, progress_bar),
                )
                .await?)
            });

            upload_handles.push(handle);
        }

        let results = futures_util::future::join_all(upload_handles).await;
        let results = results.iter().map(|r| r.as_ref().unwrap().as_ref());

        let mut err_msg = String::from("");
        for res in results {
            match res {
                Err(err) => {
                    err_msg += &format!("{err}\n");
                }
                Ok(_) => {}
            }
        }
        if err_msg.len() > 0 {
            return Err(anyhow!("WARNING: some files failed to upload!\n{err_msg}"));
        }

        Ok(())
    }

    fn get_ref_wd_from_paths<'a, I>(paths: I) -> &'a str
    where
        I: ExactSizeIterator<Item = &'a PathBuf> + Clone,
    {
        let mut paths = paths.into_iter();

        let mut ref_wd = paths.next().map(|p| p.to_str().unwrap_or("")).unwrap_or("");
        for path in paths {
            let path_str = path.to_str().unwrap();

            let mut ref_wd_split = ref_wd.split(MAIN_SEPARATOR);
            let mut path_str_split = path_str.split(MAIN_SEPARATOR);
            let mut common_i = 0;
            loop {
                let ref_seg = ref_wd_split.next();
                let path_seg = path_str_split.next();
                if path_seg.is_none() || ref_seg.is_none() {
                    break;
                }
                if path_seg != ref_seg {
                    break;
                }

                common_i += ref_seg.unwrap().len() + 1;
            }
            if common_i > 0 {
                common_i -= 1
            };

            ref_wd = &ref_wd[0..common_i];
        }

        ref_wd
    }

    fn get_zipfile_from_paths<'a, I>(&self, paths: I, password: Option<&str>) -> PathBuf
    where
        I: ExactSizeIterator<Item = &'a PathBuf> + Clone,
    {
        let paths = paths.into_iter();

        let mut output_filepath = current_dir().expect("cannot find os CWD!");

        output_filepath.push(format!(
            "tmp_upload__{}.zip",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
        ));

        let zipfile = std::fs::File::create(&output_filepath)
            .expect("error creating an upload zip file in CWD");
        let mut zipwriter = ZipWriter::new(zipfile);

        let mut options = SimpleFileOptions::default().compression_method(CompressionMethod::Zstd);
        if let Some(password) = password {
            options = options.with_aes_encryption(zip::AesMode::Aes256, password);
        }

        println!("{}", String::from("Unraveling file patterns...").bold());
        let ref_wd = Self::get_ref_wd_from_paths(paths.clone());

        println!("{}", String::from("Compressing files...").bold());
        for path in paths.clone() {
            let path_str = path.to_str().unwrap();
            let path_str = path_str
                .strip_prefix(ref_wd)
                .expect("ref working directory is not working! author skill issues...");

            if path.is_dir() {
                zipwriter
                    .add_directory(path_str, options)
                    .expect("error occured while creating directory in zipfile!");
            } else {
                zipwriter
                    .start_file(path_str, options)
                    .expect("error occured while starting file write in zipfile");

                let mut f = std::fs::File::open(&path).expect(&format!(
                    "error opening file '{}' for copy!",
                    path.to_string_lossy()
                ));
                std::io::copy(&mut f, &mut zipwriter).expect("error copying file into zipfile!");
            }
        }

        output_filepath
    }
}

impl CliSubCmd for CatCommand {
    async fn run(&self) {
        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let (url, file): (Url, Option<FsFile>) = match Url::parse(&self.location_hint) {
            Ok(url) => (url, None),
            Err(_err) => {
                let active_token = state
                    .get_active_token()
                    .expect("provided access token seems invalid!")
                    .expect("access token not found! please ensure you're logged in or have added an access token.");

                let abs_path = dirtree::get_absolute_path(&self.location_hint, wd);
                let (dirpath, filename) = dirtree::split_path(&abs_path);

                let filters = FilterGroup {
                    type_: FilterGroupType::And,
                    filters: vec![Filter(FilterCol::Name, FilterOp::Eq, json!(filename))],
                };
                let mut opts = GetFilesOpts::new(dirpath.to_string());
                opts.filters = Some(vec![filters]);

                let mut res_files = api::fs_files::get_files(Some(opts))
                    .await
                    .expect("error occured while fetching file from given path!");

                let file = res_files
                    .files
                    .pop()
                    .expect("no file found in the given path!");

                match get_share_url(Some(&active_token.0), &file.storage_id) {
                    Ok(url) => (url, Some(file)),
                    Err(_) => {
                        eprintln!("unexpected error occured while generating url!");

                        eprintln!("access token has been generated:");
                        eprintln!("{}", active_token.0.bold().cyan());
                        eprintln!();
                        eprintln!(
                            "{}",
                            format!(
                                "expires_at: {}",
                                active_token
                                    .1
                                    .expires_at
                                    .format(constants::LOCAL_DATETIME_FORMAT)
                                    .to_string()
                                    .magenta()
                            )
                            .dimmed()
                        );
                        eprintln!(
                            "{}",
                            format!("acpl: {}", active_token.1.acpl.join(", ").blue()).dimmed()
                        );
                        return;
                    }
                }
            }
        };

        let storage_id = url.path().trim_matches('/');
        let access_token = match url.query_pairs().find(|(q, _)| q.to_string() == "token") {
            Some((_, token)) => Some(token.to_string()),
            None => None,
        };

        let metadata = match file {
            Some(file) => file,
            None => get_file_metadata(storage_id, access_token.as_deref())
                .await
                .expect("error occured while fetching metadata for file!"),
        };

        let mut decryptor: Option<utils::crypto::CryptoStream<streaming::StreamOpener>> = None;
        match &metadata.encryption {
            Some(enc_metadata) => {
                if enc_metadata.attempt_decryption {
                    let password = var("PASSWORD").unwrap_or_else(|_| {
                        dialoguer::Password::new()
                            .with_prompt("File is encrypted, please enter a password:")
                            .interact()
                            .unwrap()
                    });

                    let salt = kdf::Salt::from_slice(
                        enc_metadata
                            .salt
                            .as_ref()
                            .expect("encryption metadata field missing: 'salt'"),
                    )
                    .expect("invalid encryption metadata! password salt is not valid.");
                    let key = derive_key_from_password(password.as_bytes(), &salt)
                        .expect("error occured while deriving key!");
                    let nonce = streaming::Nonce::from_slice(
                        &enc_metadata
                            .nonce
                            .as_ref()
                            .expect("encryption metadata field missing: 'header'"),
                    )
                    .expect(
                        "error occured while parsing encryption metadata! received invalid header.",
                    );
                    let e = streaming::StreamOpener::new(&key, &nonce)
                        .expect("error occured while initializing pull encrypted stream!");

                    decryptor = Some(utils::crypto::CryptoStream { e, nonce, salt })
                }

                enc_metadata.attempt_decryption
            }
            None => false,
        };

        let (_, res) = get_file_response(storage_id, access_token.as_deref())
            .await
            .expect("error occured while fetching file!");
        let mut stream = res.bytes_stream();

        let file_stream_read_buf_size = metadata
            .encryption
            .as_ref()
            .map(|e| {
                e.block_size.unwrap_or(constants::FILE_STREAM_READ_BUF_SIZE)
                    + streaming::ABYTES as u32
            })
            .unwrap_or(constants::FILE_STREAM_READ_BUF_SIZE)
            as usize;

        let progress_bar = Rc::new(ProgressBar::new(metadata.file_size as u64));

        let prog = Rc::clone(&progress_bar);
        let blocks_stream = async_stream::stream! {
            let mut data_buf: Vec<u8> = vec![];
            while let Some(chunk_res) = stream.next().await {
                let chunk = chunk_res.expect("error occured while reading stream!");
                data_buf.extend_from_slice(&chunk);
                prog.inc(chunk.len() as u64);
                drop(chunk);

                if data_buf.len() < file_stream_read_buf_size as usize {
                    continue;
                }

                let offset = (data_buf.len() / file_stream_read_buf_size) * file_stream_read_buf_size;
                let residual = data_buf.drain(offset..).collect::<Vec<u8>>();
                yield data_buf;

                data_buf = residual;
            }

            yield data_buf;
        };
        let mut blocks_stream = Box::pin(blocks_stream);

        let mut stdout = io::stdout();
        while let Some(blocks) = blocks_stream.next().await {
            let blocks_len = blocks.len();
            let mut i = 0;
            let mut c = 0;
            while i < blocks.len() {
                let slice = &blocks[i..blocks_len.min(i + file_stream_read_buf_size)];
                let slice = match &mut decryptor {
                    Some(d) => {
                        &d.e.open_chunk(slice)
                            .expect("error occured while decrypting!")
                            .0
                    }
                    None => slice,
                };

                _ = stdout
                    .write_all(slice)
                    .await
                    .expect("write to stdout failed!");
                if c % 10 == 0 {
                    if let Err(err) = stdout.flush().await {
                        eprintln!("WARNING: cannot flush stdout. error: {err}");
                    }
                }

                i += file_stream_read_buf_size;
                c += 1;
            }
        }

        progress_bar.finish_and_clear();
        _ = stdout.shutdown().await;
    }
}

impl CliSubCmd for SelectCommand {
    async fn run(&self) {
        let file_stream_read_buf_size = constants::FILE_STREAM_READ_BUF_SIZE;

        let state = STATE.read().unwrap();
        let wd = state.get_wd();

        let abs_filepath = dirtree::get_absolute_path(&self.filepath, wd);
        let (dirpath, filename) = dirtree::split_path(&abs_filepath);

        let enc = self.upload_params.get_password().map(|p| {
            utils::crypto::new_encryptor(&p).expect("error occured while initializing decryptor")
        });
        let enc_metadata = enc
            .as_ref()
            .map(|e| e.into_encryption_metadata(Some(file_stream_read_buf_size)));

        let (sender, mut receiver) = mpsc::unbounded_channel();

        let stdin = io::stdin();
        let stdin_stream = utils::streams::read_into_stream(
            stdin,
            file_stream_read_buf_size,
            enc.map(|e| e.e),
            Some(sender),
        );

        let filename = filename.to_string();
        let dirpath = dirpath.to_string();
        let upload_metadata =
            self.upload_params
                .into_upload_metadata(filename, dirpath, true, enc_metadata);
        let upload_handle = task::spawn(async move {
            api::uploads::upload_blob_stream(stdin_stream, &upload_metadata)
                .await
                .expect("error occured while uploading file!")
        });

        let is_share = self.upload_params.is_share();
        let share_exp = self.upload_params.get_share_expiry();
        let token_handle = task::spawn(async move {
            if is_share {
                match api::tokens::generate_access_token(
                    &[format!("r:{}", abs_filepath)],
                    &share_exp,
                )
                .await
                {
                    Ok(token) => Some(token),
                    Err(err) => {
                        eprintln!("error occured while generating access token! {}", err);
                        None
                    }
                }
            } else {
                None
            }
        });

        let progress_bar = ProgressBar::new_spinner().with_style(
            ProgressStyle::with_template("[{elapsed}] {spinner} {binary_bytes} ({bytes_per_sec})")
                .unwrap(),
        );
        while let Some(b) = receiver.recv().await {
            progress_bar.inc(b as u64);
        }

        let (file, token) = futures_util::future::join(upload_handle, token_handle).await;
        progress_bar.finish();

        let file = file.expect("error occured while uploading file!");
        let token = token.expect("error occured while generating token!");

        if let Some(token_res) = token {
            let access_token: shared_types::AccessToken = token_res
                .access_token
                .parse()
                .expect("invalid access token returned! cannot be parsed.");
            let url = get_share_url(
                match self.upload_params.public {
                    true => None,
                    false => Some(&token_res.access_token),
                },
                &file.storage_id,
            )
            .expect("failed to generate url!");

            eprintln!("\n{}\n", url.to_string().bold().cyan());
            eprintln!(
                "{}",
                format!("acpl: {}", access_token.acpl.join(", ").blue()).dimmed()
            );
            eprintln!(
                "{}",
                format!(
                    "expires at: {}",
                    access_token
                        .expires_at
                        .format(constants::LOCAL_DATETIME_FORMAT)
                        .to_string()
                        .magenta()
                )
                .dimmed()
            );
        }
    }
}

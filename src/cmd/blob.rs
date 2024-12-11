use std::{
    env::{current_dir, var},
    io,
    os::unix::fs::MetadataExt,
    path::{PathBuf, MAIN_SEPARATOR},
};

use anyhow::anyhow;
use clap::Parser;
use colored::*;
use futures_util::StreamExt;
use tokio::{fs, io::AsyncWriteExt};
use url::Url;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    api::{self, fs_files::*},
    config::{CliConfig, LogLevel, CONFIG},
    constants::{self, MIME_TYPES},
    shared_types::{self, CliSubCmd, UploadSingleBlogMetadata},
    utils::{
        chirpy_logs,
        crypto::{self, Decrypter},
        dirtree, files,
        local_auth::LocalAuthData,
        paths, tokens,
    },
};

#[derive(Parser)]
pub struct UploadBlobCommand {
    /// multiple files and/or directories are uploaded as a zipfile, single file as-is.
    source_path_patterns: Vec<String>,

    #[arg(long)]
    /// remote dir path. by default, CWD set by you in this cli, will be used as remote dirpath to upload file(s).
    dirpath: Option<String>,

    #[arg(long)]
    /// mark the upload public, anyone with the file id can view, no access token required
    public: bool,

    #[arg(long, short)]
    /// relative, absolute or just name of the output zip file. (has no effect for non-encrypted single file uploads)
    output: Option<String>,

    #[arg(long)]
    /// take password input in a 'PASSWORD' env variable or from a text prompt
    password: bool,

    #[arg(long, short)]
    /// even if a remote file already exists with the same name in the dirpath, it will forcefully write (update)
    force: bool,

    #[arg(long, short)]
    /// the file will be saved as this name in "dirpath" after upload (has no effect on multiple file uploads)
    name: Option<String>,

    #[arg(long)]
    /// generates a read-only access token embedded url for the upload. url is valid for 30mins. (has no effect on public uploads)
    share: bool,

    #[arg(long)]
    /// used in conjunction with 'share', provide custom expiry time for the share url. (e.g "AAdBBhCCmDDs", "7d30s", "2h30m12s", "30m" [default])
    share_exp: Option<String>,
}

#[derive(Parser)]
pub struct GetBlobCommand {
    /// gimme anything, remote file path relative or absolute or a file url with/without embedded token
    location_hints: Vec<String>,

    #[arg(short, long)]
    /// store the downloaded file at this path on your machine. can be relative. defaults to CWD
    output: Option<String>,
}

impl CliSubCmd for UploadBlobCommand {
    async fn run(&self) {
        let paths: Vec<PathBuf> = self
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

        let config = CONFIG
            .try_lock()
            .expect("CONFIG lock connot be acquired, author skill issues...");

        let wd = config.get_wd();

        let remote_dirpath = match &self.dirpath {
            Some(dirpath) => dirtree::get_absolute_path(dirpath, wd),
            None => wd.to_string(),
        }
        .to_string();

        let password: Option<String> = match var("PASSWORD") {
            Ok(pwd) => Some(pwd),
            Err(_) => match self.password {
                true => Some(
                    dialoguer::Password::new()
                        .with_prompt("Create zipfile encryption password")
                        .with_confirmation("Confirm", "Passwords don't match!")
                        .interact()
                        .unwrap(),
                ),
                false => None,
            },
        };

        let single_file_upload = paths.len() == 1;
        let metadata_is_encrypted = password.is_some() && single_file_upload;

        let mut upload_filepath = paths[0].clone();
        if !single_file_upload {
            upload_filepath = self.get_zipfile_from_paths(&paths, password.as_deref());
        }

        let mut url = api::get_base_url(&config)
            .expect("error occured while generating a url to fetch from!");

        let mut upload_file = fs::File::open(&upload_filepath)
            .await
            .expect("error occured while opening file for upload!");
        let metadata = upload_file
            .metadata()
            .await
            .expect("error occured while reading file metadata!");

        let file_ext = upload_filepath
            .to_str()
            .unwrap_or("")
            .split(".")
            .last()
            .unwrap_or("");
        let file_type = constants::MIME_TYPES
            .get(file_ext)
            .unwrap_or(&constants::UNKNOWN_MIME_TYPE);

        let filename = match upload_filepath.file_stem() {
            Some(filename) => Some(filename.to_string_lossy().to_string()),
            None => None,
        };
        let filename = match &self.name {
            Some(name) => Some(name.clone()),
            None => filename,
        };

        if metadata.size() < constants::MIN_MULTIPART_UPLOAD_SIZE as u64 {
            url.set_path("/blob/upload");

            let upload_metadata = UploadSingleBlogMetadata {
                name: filename.clone(),
                is_public: self.public,
                is_encrypted: metadata_is_encrypted,
                dir_path: remote_dirpath,
                file_type: file_type.to_string(),
                force_write: self.force,
            };

            let fs_file = api::uploads::upload_file(
                &config,
                &upload_metadata,
                &mut upload_file,
                match metadata_is_encrypted {
                    true => password.as_deref(),
                    false => None,
                },
            )
            .await
            .expect("error occured while uploading file!");

            match config.get_log_level() {
                LogLevel::Chirpy => chirpy_logs::recalldeezai_product_placement(),
                _ => {}
            };

            if !single_file_upload {
                drop(upload_file);
                _ = fs::remove_file(&upload_filepath).await;
            }

            let auth_data = LocalAuthData::get().unwrap().unwrap();

            let share_url: String;
            if self.public {
                share_url = files::get_share_url(None, &fs_file.storage_id, &config)
                    .expect("error occured while generating share url!");
            } else if self.share {
                let access_token_ttl = tokens::parse_validate_ttl(match &self.share_exp {
                    Some(exp) => exp.as_str(),
                    None => "30m",
                })
                .expect("invalid share expiry time provided!");

                let acpl: Vec<String> = vec![tokens::get_acpl(
                    shared_types::AccessTokenPermission::ReadPrivate,
                    &format!("{}/{}", upload_metadata.dir_path, fs_file.name),
                )];
                let expires_at = chrono::Utc::now() + access_token_ttl;

                let access_token = api::tokens::generate_access_token(&config, &acpl, expires_at)
                    .await
                    .expect("error occured while generating access token!");

                share_url =
                    files::get_share_url(Some(&access_token.token), &fs_file.storage_id, &config)
                        .expect("error occured while generating share url!");
            } else {
                share_url = files::get_share_url(
                    Some(&auth_data.access_token.token),
                    &fs_file.storage_id,
                    &config,
                )
                .expect("error occured while generating share url!");

                println!(
                    "{}",
                    String::from(
                        "this link is only for private view, not to be shared with anyone!"
                    )
                    .bold()
                );
                println!(
                    "{}",
                    String::from(
                        "to share, either make this file public or generate a shareable private link."
                    )
                    .bold()
                );
            }

            println!("\n{}\n", share_url.blue());

            println!(
                "{}",
                format!(
                    "Full path: {}",
                    upload_metadata.dir_path + "/" + &fs_file.name
                )
                .dimmed()
            );
        } else {
            todo!("multipart upload not implemented yet!");
        }
    }
}

impl UploadBlobCommand {
    fn get_zipfile_from_paths(&self, paths: &Vec<PathBuf>, password: Option<&str>) -> PathBuf {
        let mut output_filepath =
            current_dir().expect("what kind of sadass machine is this? CWD NOT FOUND OR INVALID!");

        if let Some(output_filepath_str) = &self.output {
            output_filepath = paths::get_absolute_path(output_filepath_str)
                .expect("error occured while parsing output path to absolute path!")
                .with_extension("zip");

            if let Some(output_filepath_parent) = output_filepath.parent() {
                let _ = fs::create_dir_all(output_filepath_parent);
            }
        } else {
            output_filepath.push(format!(
                "upload__{}.zip",
                chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
            ));
        }

        let zipfile = std::fs::File::create(&output_filepath)
            .expect("error creating an upload zip file in CWD");
        let mut zipwriter = ZipWriter::new(zipfile);

        let mut options = SimpleFileOptions::default().compression_method(CompressionMethod::Zstd);
        if let Some(password) = password {
            options = options.with_aes_encryption(zip::AesMode::Aes256, password);
        }

        println!("{}", String::from("Unraveling file patterns...").bold());
        let mut ref_wd = paths[0].to_str().unwrap();
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

        println!("{}", String::from("Compressing files...").bold());
        for path in paths {
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

                let mut f = std::fs::File::open(&path).expect("error opening file for copy!");
                io::copy(&mut f, &mut zipwriter).expect("error copying file into zipfile!");
            }
        }

        output_filepath
    }
}

async fn get_file_url_from_path(
    config: &CliConfig,
    dirtree: &shared_types::DirTree,
    path_hint: &str,
) -> anyhow::Result<Url> {
    let abs_path = dirtree::get_absolute_path(path_hint, config.get_wd());
    let (dirtree, filename) = match dirtree.split_path(&abs_path) {
        Some(result) => {
            let (dirtree, _) = result;
            match result.1 {
                Some(filename) => (dirtree, filename),
                None => return Err(anyhow!("provided dir path is not a file!")),
            }
        }
        None => return Err(anyhow::anyhow!("no matching valid path found!")),
    };
    let mut filename = filename.split('.').collect::<Vec<&str>>();
    if filename.len() > 1 {
        filename.pop();
    }
    let filename = filename.join(".");

    let get_files_opts = GetFilesOpts {
        filters: Some(vec![Filter(
            FilterCol::Name,
            FilterOp::Eq,
            serde_json::json!(&filename),
        )]),
        limit: None,
        page: None,
        order_by: None,
        order: None,
    };

    let result = get_files(config, &dirtree.id, Some(get_files_opts)).await?;
    match result.files.first() {
        Some(file) => {
            let mut url = api::get_base_url(config)?;
            url.set_path(&file.storage_id);

            Ok(url)
        }
        None => Err(anyhow::anyhow!("file not found!")),
    }
}

impl CliSubCmd for GetBlobCommand {
    async fn run(&self) {
        let config = CONFIG.try_lock().unwrap();

        for location_hint in &self.location_hints {
            let url = match Url::parse(location_hint) {
                Ok(url) => url,
                Err(_err) => {
                    let res = api::dirtree::get_dirtree(&config)
                        .await
                        .expect("error occured while fetching your dirtree!");

                    get_file_url_from_path(&config, &res.dirtree, &location_hint)
                        .await
                        .expect("error occured while fetching file from given path!")
                }
            };

            let storage_id = url.path().trim_matches('/');
            let access_token = match url.query_pairs().find(|(q, _)| q.to_string() == "token") {
                Some((_, token)) => Some(token.to_string()),
                None => None,
            };

            let metadata = get_file_metadata(&config, storage_id, access_token.as_deref())
                .await
                .expect("error occured while fetching metadata for file!");

            let file_ext =
                MIME_TYPES
                    .entries()
                    .find_map(|(k, v)| match **v == metadata.file_type {
                        true => Some(k.to_string()),
                        false => None,
                    });
            let filepath = match &self.output {
                None => current_dir()
                    .expect("CWD NOT FOUND WTF?")
                    .join(metadata.name),
                Some(path) => {
                    let mut abs_path = paths::get_absolute_path(path)
                        .expect("provided output path seems invalid!");

                    if abs_path.file_stem().is_some() {
                        abs_path = abs_path.join(metadata.name);
                    }

                    abs_path
                }
            }
            .with_extension(match file_ext {
                Some(file_ext) => file_ext.to_string(),
                None => String::new(),
            });

            let mut file_opts = fs::File::options();
            file_opts.append(true).create_new(true);

            let mut file = file_opts
                .open(&filepath)
                .await
                .expect("error occured while opening file in append mode!");

            let mut password: Option<String> = None;
            if metadata.is_encrypted {
                if let Ok(pwd) = var("PASSWORD") {
                    password = Some(pwd);
                } else {
                    password = Some(
                        dialoguer::Password::new()
                            .with_prompt("File is encrypted, please enter a password:")
                            .interact()
                            .unwrap(),
                    );
                }
            }

            let (metadata, res) = get_file_response(&config, storage_id, access_token.as_deref())
                .await
                .expect("error occured while fetching file!");

            let mut stream = res.bytes_stream();
            let mut _first = true;
            let mut decrypter: Option<Decrypter> = None;
            while let Some(Ok(data)) = stream.next().await {
                let data = data.to_vec();
                let mut data_read_buf = data.as_slice();

                if metadata.is_encrypted {
                    if _first {
                        if data_read_buf.len() < crypto::HEADER_LENGTH {
                            println!(
                                "{}",
                                "File is encrypted with invalid encryption, cannot be retreived!"
                                    .red()
                            );
                            return;
                        }

                        let cipher_header: [u8; crypto::HEADER_LENGTH] = data_read_buf
                            [0..crypto::HEADER_LENGTH]
                            .try_into()
                            .expect("error occured while getting encryption header!");
                        data_read_buf = &data_read_buf[crypto::HEADER_LENGTH..];

                        decrypter = Some(
                            Decrypter::new(password.as_deref().unwrap(), cipher_header)
                                .expect("error occured while generating decrypter!"),
                        );

                        _first = false;
                    }

                    let decrypter = decrypter.as_ref().unwrap();

                    let plaintext = decrypter
                        .decrypt(&data_read_buf)
                        .expect("Decryption error occured! Please check your password.");

                    let b_write = file
                        .write(&plaintext)
                        .await
                        .expect("file incorrectly written, error occured while writing.");
                    if b_write < plaintext.len() {
                        println!(
                            "WARNING: only {} of {} bytes written in {}",
                            b_write,
                            plaintext.len(),
                            filepath.to_str().unwrap()
                        );
                    }
                } else {
                    let b_write = file
                        .write(&data_read_buf)
                        .await
                        .expect("file incorrectly written, error occured while writing.");
                    if b_write < data_read_buf.len() {
                        println!(
                            "WARNING: only {} of {} bytes written in {}",
                            b_write,
                            data_read_buf.len(),
                            filepath.to_str().unwrap()
                        );
                    }
                }
            }
        }

        println!("Downloaded files successfully.");
    }
}

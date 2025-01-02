use std::{
    env::{current_dir, var},
    io,
    os::unix::fs::MetadataExt,
    path::{PathBuf, MAIN_SEPARATOR},
    str::FromStr,
};

use chrono::Duration;
use clap::Parser;
use colored::*;
use futures_util::StreamExt;
use inquire::Confirm;
use serde_json::json;
use tokio::{fs, io::AsyncWriteExt};
use url::Url;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    api::{self, fs_files::*},
    config::{LogLevel, CONFIG},
    constants::{self, MIME_TYPES, UNKNOWN_MIME_TYPE, ZIPFILE_MIME_TYPE},
    shared_types::{
        AccessTokenPermission, AppContext, CliSubCmd, FsFile, PermissionChar,
        UploadSingleBlogMetadata,
    },
    state::STATE,
    utils::{
        chirpy_logs,
        crypto::{self, Decrypter},
        dirtree,
        files::{self, get_share_url},
        local_auth::LocalAuthData,
        paths, str2x, tokens,
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

    #[arg(long, value_parser = str2x::str2duration)]
    /// set the 'max-age' value for cache, defaults to 0 (format: 12d23h34m45s)
    max_age: Option<Duration>,

    #[arg(long)]
    /// do not display confirm prompt when uploading multiple files matching with provided path pattern
    no_confirm: bool,
}

#[derive(Parser)]
pub struct GetBlobCommand {
    /// gimme anything, remote file path relative or absolute or a file url with/without embedded token
    location_hint: String,

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

        println!(
            "{}",
            format!("Files covered by pattern: {}", paths.len()).bold()
        );

        if paths.len() > 1 {
            let (ref_wd, pretty_paths) = paths::get_pretty_paths(&paths);

            println!();
            if ref_wd.len() > 0 {
                println!("{}", ref_wd.bold().dimmed());
            }
            println!("{}", pretty_paths.dimmed().blue());
            println!();

            if !self.no_confirm {
                let confirm = Confirm::new(&format!(
                    "{} files matched the pattern, confirm to upload",
                    paths.len()
                ))
                .with_default(false)
                .prompt()
                .expect("error occured while displaying confirm prompt!");

                if !confirm {
                    println!("Aborted upload.");
                    return;
                }
            }
        }

        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };
        let wd = ctx.config.get_wd();

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

        let mut upload_filepath = paths[0].clone();
        if !single_file_upload {
            upload_filepath = self.get_zipfile_from_paths(&paths, password.as_deref());
        }

        let mut upload_file = fs::File::open(&upload_filepath)
            .await
            .expect("error occured while opening file for upload!");
        let metadata = upload_file
            .metadata()
            .await
            .expect("error occured while reading file metadata!");

        let file_ext = files::get_file_ext(
            upload_filepath
                .to_str()
                .expect("invalid file path provided!"),
        );
        let file_type = constants::MIME_TYPES
            .get(file_ext)
            .unwrap_or(&constants::UNKNOWN_MIME_TYPE);

        let filename = match &self.name {
            Some(name) => Some(name.to_string()),
            None => match upload_filepath.file_stem() {
                Some(filename) => Some(filename.to_string_lossy().to_string()),
                None => None,
            },
        };

        if metadata.size() < constants::MIN_MULTIPART_UPLOAD_SIZE as u64 {
            let upload_metadata = UploadSingleBlogMetadata {
                name: filename.clone(),
                is_public: self.public,
                is_encrypted: password.is_some(),
                dir_path: remote_dirpath,
                file_type: file_type.to_string(),
                force_write: self.force,
                cache_max_age_seconds: match self.max_age {
                    Some(max_age) => Some(max_age.num_seconds().abs() as u64),
                    None => None,
                },
            };

            let fs_file = api::uploads::upload_file(
                &ctx,
                &upload_metadata,
                &mut upload_file,
                match single_file_upload {
                    true => password.as_deref(),
                    false => None,
                },
            )
            .await
            .expect("error occured while uploading file!");

            if password.is_some() {
                println!(
                    "{}",
                    String::from("PLEASE ENSURE YOU REMEMBER/SAVE THIS PASSWORD!").bold()
                );

                match ctx.config.get_log_level() {
                    LogLevel::Chirpy => chirpy_logs::recalldeezai_product_placement(),
                    _ => {}
                };
            }

            if !single_file_upload {
                drop(upload_file);
                _ = fs::remove_file(&upload_filepath).await;
            }

            let auth_data = LocalAuthData::get()
                .expect("You're not logged in! please login before uploading files.");

            let share_url: String;
            if self.public {
                share_url = files::get_share_url(None, &fs_file.storage_id, &ctx)
                    .expect("error occured while generating share url!")
                    .to_string();
            } else if self.share {
                let access_token_ttl = str2x::str2duration(match &self.share_exp {
                    Some(exp) => exp.as_str(),
                    None => "30m",
                })
                .expect("invalid share expiry time provided!");

                let acpl: Vec<String> = vec![tokens::get_acp(
                    AccessTokenPermission::from_str(&PermissionChar::Read.to_string())
                        .expect("Error occured while parsing read access token permission"),
                    &format!("{}/{}", upload_metadata.dir_path, fs_file.name),
                )];
                let expires_at = chrono::Utc::now() + access_token_ttl;

                let res_data = api::tokens::generate_access_token(&ctx, &acpl, &expires_at)
                    .await
                    .expect("error occured while generating access token!");

                share_url =
                    files::get_share_url(Some(&res_data.access_token), &fs_file.storage_id, &ctx)
                        .expect("error occured while generating share url!")
                        .to_string();
            } else {
                share_url =
                    files::get_share_url(Some(&auth_data.access_token), &fs_file.storage_id, &ctx)
                        .expect("error occured while generating share url!")
                        .to_string();

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
                    dirtree::join_paths(vec![&upload_metadata.dir_path, &fs_file.name]),
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

impl CliSubCmd for GetBlobCommand {
    async fn run(&self) {
        let ctx = AppContext {
            config: &CONFIG.try_lock().unwrap(),
            state: &STATE.try_lock().unwrap(),
        };

        let wd = ctx.config.get_wd();

        let (url, file): (Url, Option<FsFile>) = match Url::parse(&self.location_hint) {
            Ok(url) => (url, None),
            Err(_err) => {
                let active_token = ctx.state
                    .get_active_token()
                    .expect("provided access token seems invalid!")
                    .expect("access token not found! please ensure you're logged in or have added an access token.");

                let abs_path = dirtree::get_absolute_path(&self.location_hint, wd);
                let (dirpath, filename) = dirtree::split_path(&abs_path);

                let filters = vec![Filter(FilterCol::Name, FilterOp::Eq, json!(filename))];
                let opts = GetFilesOpts::new(dirpath, Some(&filters));
                let mut res_files = api::fs_files::get_files(&ctx, Some(opts))
                    .await
                    .expect("error occured while fetching file from given path!");

                let file = res_files
                    .files
                    .pop()
                    .expect("no file found in the given path!");

                match get_share_url(Some(&active_token.0), &file.storage_id, &ctx) {
                    Ok(url) => (url, Some(file)),
                    Err(_) => {
                        println!("unexpected error occured while generating url!");

                        println!("access token has been generated:");
                        println!("{}", active_token.0.bold().cyan());
                        println!();
                        println!(
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
                        println!(
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
            Some(file) => PublicFileMetadata {
                name: file.name,
                is_encrypted: file.is_encrypted,
            },
            None => get_file_metadata(&ctx, storage_id, access_token.as_deref())
                .await
                .expect("error occured while fetching metadata for file!"),
        };

        let filepath = match &self.output {
            None => current_dir()
                .expect("CWD NOT FOUND WTF?")
                .join(&metadata.name),
            Some(path) => {
                let mut abs_path =
                    paths::get_absolute_path(path).expect("provided output path seems invalid!");

                if !abs_path.is_file() {
                    abs_path = abs_path.join(&metadata.name);
                }

                abs_path
            }
        };

        let mut file_opts = fs::File::options();
        file_opts.append(true).create_new(true);

        let mut file = file_opts
            .open(&filepath)
            .await
            .expect("error occured while opening file in append mode!");

        let is_zip = MIME_TYPES
            .get(files::get_file_ext(&metadata.name))
            .unwrap_or(&UNKNOWN_MIME_TYPE)
            == &ZIPFILE_MIME_TYPE;
        let should_decrypt = metadata.is_encrypted && !is_zip;
        let mut password: Option<String> = None;
        if should_decrypt {
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

        let (_, res) = get_file_response(&ctx, storage_id, access_token.as_deref())
            .await
            .expect("error occured while fetching file!");

        let mut stream = res.bytes_stream();
        let mut _first = true;
        let mut decrypter: Option<Decrypter> = None;
        while let Some(Ok(data)) = stream.next().await {
            let data = data.to_vec();
            let mut data_read_buf = data.as_slice();

            if should_decrypt {
                if _first {
                    if data_read_buf.len() < crypto::HEADER_LENGTH {
                        println!(
                            "{}",
                            "File is encrypted with invalid encryption, cannot be retreived!".red()
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

        println!("Downloaded files successfully.");
    }
}

use clap::{Parser, Subcommand};
use cmd::blob;
use cmd::metadata::MetadataCommand;
use cmd::tokens::TokensCommand;
use cmd::usage::UsageCommand;
use colored::Colorize;
use utils::local_auth::LocalAuthData;

mod api;
mod cmd;
mod config;
mod constants;
mod shared_types;
mod state;
mod utils;

use crate::cmd::auth::AuthCommand;
use crate::cmd::config::ConfigCommand;
use crate::cmd::dirtree;
use crate::cmd::serve::ServeCommand;
use crate::shared_types::CliSubCmd;

#[derive(Parser)]
#[command(about = "CLI to manage your SFS file system.")]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// log into your SFS account
    Auth(AuthCommand),
    /// manage (generate/blacklist) access tokens
    Tokens(TokensCommand),
    /// utility command to serve local files on the local network matching given path pattern
    Serve(ServeCommand),
    /// manage local CLI config
    Config(ConfigCommand),
    /// get your api key usage (you need to be the file system owner to run this command)
    Usage(UsageCommand),

    /// list remote files in a remote directory (default: currently selected WD)
    Ls(dirtree::LsCommand),
    /// print remote directory structure
    Tree(dirtree::TreeCommand),
    /// create an empty file in a remote directory (default: currently selected WD)
    Touch(dirtree::TouchCommand),
    /// select a file to push contents into. (e.g. echo "Hello, World!" | sfs select "./hw.txt")
    Select(blob::SelectCommand),
    /// print remote file's contents to stdout. use '>' to redirect to a file
    Cat(blob::CatCommand),
    /// remove a remote file
    Rm(dirtree::RmCommand),
    /// move a remote file to a different remote location
    Mv(dirtree::MvCommand),
    /// create a remote directory
    Mkdir(dirtree::MkdirCommand),
    /// remove a remote directory
    Rmdir(dirtree::RmdirCommand),
    /// move a remote directory to a new remote location
    Mvdir(dirtree::MvdirCommand),
    /// print currently selected remote working directory (WD)
    Pwd(dirtree::PwdCommand),
    /// change currently selected WD
    Cd(dirtree::Cd),

    /// get a sharable url for a remote file
    Url(dirtree::UrlCommand),
    /// upload local file(s) into a remote file directory
    Upload(blob::UploadBlobCommand),
    /// manage metadata for remote files
    Metadata(MetadataCommand),
}

#[tokio::main]
pub async fn main() {
    let cli = Cli::parse();

    LocalAuthData::load().expect("error occured while initializing auth!");

    match cli.commands {
        Commands::Serve(_cmd) => _cmd.run().await,
        Commands::Config(_cmd) => _cmd.run().await,
        Commands::Auth(_cmd) => _cmd.run().await,
        Commands::Metadata(_cmd) => _cmd.run().await,
        Commands::Tokens(_cmd) => _cmd.run().await,
        Commands::Usage(_cmd) => _cmd.run().await,

        // fs commands
        Commands::Select(_cmd) => _cmd.run().await,
        Commands::Mkdir(_cmd) => _cmd.run().await,
        Commands::Rmdir(_cmd) => _cmd.run().await,
        Commands::Rm(_cmd) => _cmd.run().await,
        Commands::Mvdir(_cmd) => _cmd.run().await,
        Commands::Mv(_cmd) => _cmd.run().await,
        Commands::Tree(_cmd) => _cmd.run().await,
        Commands::Cd(_cmd) => _cmd.run().await,
        Commands::Pwd(_cmd) => _cmd.run().await,
        Commands::Ls(_cmd) => _cmd.run().await,
        Commands::Touch(_cmd) => _cmd.run().await,
        Commands::Url(_cmd) => _cmd.run().await,

        // blob commands
        Commands::Upload(_cmd) => _cmd.run().await,
        Commands::Cat(_cmd) => _cmd.run().await,
    };
}

use clap::{Parser, Subcommand};
use cmd::blob;
use cmd::metadata::MetadataCommand;

mod api;
mod cmd;
mod config;
mod constants;
mod shared_types;
mod utils;

use crate::cmd::auth::AuthCommand;
use crate::cmd::config::ConfigCommand;
use crate::cmd::dirtree;
use crate::cmd::serve::ServeCommand;
use crate::shared_types::CliSubCmd;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve(ServeCommand),
    Auth(AuthCommand),
    Config(ConfigCommand),
    Metadata(MetadataCommand),

    // fs commands
    Mkdir(dirtree::MkdirCommand),
    Rmdir(dirtree::RmdirCommand),
    Rm(dirtree::RmCommand),
    Mvdir(dirtree::MvdirCommand),
    Tree(dirtree::TreeCommand),
    Setwd(dirtree::SetwdCommand),
    Pwd(dirtree::PwdCommand),
    Ls(dirtree::LsCommand),

    // blob commands
    Upload(blob::UploadBlobCommand),
    Get(blob::GetBlobCommand),
}

#[tokio::main]
pub async fn main() {
    let cli = Cli::parse();
    match cli.commands {
        Commands::Serve(_cmd) => _cmd.run().await,
        Commands::Config(_cmd) => _cmd.run().await,
        Commands::Auth(_cmd) => _cmd.run().await,
        Commands::Metadata(_cmd) => _cmd.run().await,

        // fs commands
        Commands::Mkdir(_cmd) => _cmd.run().await,
        Commands::Rmdir(_cmd) => _cmd.run().await,
        Commands::Rm(_cmd) => _cmd.run().await,
        Commands::Mvdir(_cmd) => _cmd.run().await,
        Commands::Tree(_cmd) => _cmd.run().await,
        Commands::Setwd(_cmd) => _cmd.run().await,
        Commands::Pwd(_cmd) => _cmd.run().await,
        Commands::Ls(_cmd) => _cmd.run().await,

        // blob commands
        Commands::Upload(_cmd) => _cmd.run().await,
        Commands::Get(_cmd) => _cmd.run().await,
    };
}

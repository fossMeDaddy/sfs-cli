use clap::{Parser, Subcommand};
use colored::*;
use inquire::Text;
use serde::Deserialize;

use crate::{
    api,
    config::CONFIG,
    shared_types::{AccessToken, ApiKey, CliSubCmd},
    utils::local_auth,
};

#[derive(Parser)]
pub struct AuthCommand {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// login using github
    Login,
    Logout,
}

impl AuthCommand {
    async fn handle_login(&self) {
        let config = CONFIG.try_lock().unwrap();

        println!(
            "Please copy and paste this oauth url in your browser:\n{}\n",
            config.get_gh_login_uri().blue(),
        );

        print!("{}", String::from("Note: ").bright_black());
        print!(
            "{}",
            String::from("we use OS keyrings to save access tokens. ").bright_black()
        );
        print!(
            "{}",
            String::from(
                "Don't forget to set always allow on our keyring requests or it could get annoying... uwu"
            )
            .bright_black()
        );
        println!();

        let code = Text::new("Enter login code:").prompt().unwrap();

        #[derive(Deserialize)]
        struct Res {
            access_tokens: AccessToken,
            api_keys: ApiKey,
        }

        let mut url = api::get_base_url(&config).expect("invalid url provided!");
        url.set_path("auth/gh-cli-login");
        url.set_query(Some(format!("key={}", code)).as_deref());

        let req = api::get_builder(reqwest::Method::GET, url)
            .unwrap()
            .send()
            .await
            .unwrap();
        let response: Res = req.json().await.unwrap();

        let local_auth_data = local_auth::LocalAuthData {
            access_token: response.access_tokens,
            api_key: response.api_keys,
        };
        local_auth_data
            .save()
            .expect("Error occured while saving auth data in keyring!");

        println!("Successfully logged in!")
    }

    fn handle_logout(&self) {
        local_auth::LocalAuthData::delete().expect("Error occured while deleting keyring entry");

        println!("Successfully logged out!");
    }
}

impl CliSubCmd for AuthCommand {
    async fn run(&self) {
        match &self.commands {
            Commands::Login => self.handle_login().await,
            Commands::Logout => self.handle_logout(),
        };
    }
}

use chrono::{DateTime, Duration, Local, Utc};
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Matcher,
};

use crate::{
    api,
    config::CONFIG,
    constants,
    shared_types::{AccessToken, AppContext, CliSubCmd},
    state::{ActiveToken, STATE},
    utils::{self, dirtree::PrintDirTreeOpts, str2x},
};

#[derive(Parser)]
pub struct TokensCommand {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// generate access tokens to grant fine-grained access to other people. use `url` command to get shareable urls for individual objects
    Generate {
        /// enter comma-separated ACPLs e.g. "read_private:/tmp/**", "files_owner:/project1/**/*.{bin,exe}", etc.
        acpl: Vec<String>,

        #[arg(short, long)]
        /// save this access token locally with a tag name
        tag: Option<String>,

        #[command(flatten)]
        exp_input: ExpInput,
    },

    /// list all tagged access tokens saved locally
    Ls {
        #[arg(short, long)]
        /// name of tag (fuzzy searched)
        tag: Option<String>,
    },

    /// switch between access tokens to access different FileSystems shared by different people
    Use {
        /// locally saved access token's tag name or full access token. leave blank to use your account's root access token
        input: Option<String>,
    },

    /// input a list of tokens to blacklist in case you need to revoke access of a user
    Blacklist { tokens: Vec<String> },
}

#[derive(Args)]
#[group(multiple = false)]
pub struct ExpInput {
    #[arg(long, value_parser = str2x::str2datetime)]
    /// provide an expiry datetime in your local timezone. format: %Y-%m-%d %H:%M:%S
    expires_at: Option<DateTime<Local>>,

    #[arg(long, value_parser = str2x::str2duration)]
    /// provide a duration. format: 1d2h3m4s, default: 30m
    ttl: Option<Duration>,
}

impl ExpInput {
    pub fn get_expires_at(&self) -> DateTime<Utc> {
        match self.expires_at {
            Some(exp) => exp.to_owned().into(),
            None => (Utc::now()
                + match self.ttl {
                    Some(ttl) => ttl.to_owned(),
                    None => Duration::minutes(30),
                })
            .into(),
        }
    }
}

impl CliSubCmd for TokensCommand {
    async fn run(&self) {
        match &self.command {
            Commands::Generate {
                acpl,
                tag,
                exp_input,
            } => handle_generate(&acpl, tag.as_deref(), &exp_input).await,
            Commands::Ls { tag: tagname } => handle_list_tokens(tagname.as_deref()).await,
            Commands::Blacklist { tokens } => handle_blacklist_token(tokens).await,
            Commands::Use { input } => handle_use_token(input.as_deref()).await,
        }
    }
}

pub async fn handle_use_token<S>(input: Option<S>)
where
    S: AsRef<str>,
{
    let mut state = STATE.lock().unwrap();
    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);

    let prev_active_token = state.active_token.clone();

    match input {
        None => {
            _ = state.guard_mutate(|state| {
                state.active_token = ActiveToken::RootAccessToken;
                Ok(())
            });
        }
        Some(token_or_tag) => {
            match token_or_tag.as_ref().parse::<AccessToken>() {
                Ok(access_token) => {
                    let token = token_or_tag.as_ref();

                    if access_token.expires_at.lt(&Local::now()) {
                        _ = state.guard_mutate(|state| {
                            state.tokens.remove(token);

                            Ok(())
                        });
                        println!("{}", "token has expired!".to_string().bold());
                        return;
                    }

                    _ = state.guard_mutate(|state| {
                        let new_tag = state.get_untitled_token_tag();
                        state.tokens.insert(new_tag.clone(), token.to_string());

                        state.active_token = ActiveToken::Tag(new_tag);
                        Ok(())
                    });
                }
                Err(_) => {
                    let tag = token_or_tag.as_ref();

                    match state.tokens.get(tag) {
                        None => {
                            println!("cannot find a locally saved tag with name '{tag}'");

                            let matches = Pattern::new(
                                tag,
                                CaseMatching::Ignore,
                                Normalization::Never,
                                AtomKind::Fuzzy,
                            )
                            .match_list(state.tokens.keys(), &mut matcher)
                            .iter()
                            .take(7)
                            .map(|(m, _)| format!("'{m}'"))
                            .collect::<Vec<String>>();

                            if matches.len() > 0 {
                                println!("did you mean: {}", matches.join(", "));
                            }

                            return;
                        }
                        Some(access_token) => {
                            let access_token: AccessToken = access_token
                                .parse()
                                .expect("access token for this tag seems invalid!");
                            if access_token.expires_at.lt(&Local::now()) {
                                _ = state.guard_mutate(|state| {
                                    state.tokens.remove(tag);
                                    Ok(())
                                });
                                println!("{}", "access token has expired!".to_string().bold());
                                return;
                            }

                            _ = state.guard_mutate(|state| {
                                state.active_token = ActiveToken::Tag(tag.to_string());
                                Ok(())
                            });
                        }
                    };
                }
            };
        }
    };

    let active_token = &state.active_token;
    println!("'{prev_active_token}' -> '{active_token}'");
}

pub async fn handle_list_tokens<S>(tagname: Option<S>)
where
    S: AsRef<str>,
{
    let mut state = STATE.lock().unwrap();

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);

    let matches = match tagname {
        Some(tagname) => Pattern::new(
            tagname.as_ref(),
            CaseMatching::Ignore,
            Normalization::Never,
            AtomKind::Fuzzy,
        )
        .match_list(state.tokens.keys(), &mut matcher)
        .iter()
        .map(|m| m.0)
        .collect::<Vec<&String>>(),

        None => state.tokens.keys().collect::<Vec<&String>>(),
    };

    if matches.len() == 0 {
        println!("no tokens saved locally, generate tokens with a 'tag' to save them locally.");
        return;
    }

    struct OutputData {
        name: String,
        exp: String,
        acpl: String,
        status: String,
    }
    let mut name_padding = 0;
    let mut acpl_padding = 0;
    let mut exp_padding = 0;
    let mut exp_keys: Vec<String> = vec![];
    let mut output_tuples: Vec<OutputData> = vec![];

    for k in matches {
        if let Some(val) = state.tokens.get(k) {
            let access_token: AccessToken = val
                .parse()
                .expect(&format!("error occured while parsing access token: {val}"));

            if access_token.expires_at.lt(&Local::now()) {
                exp_keys.push(k.to_string());
                continue;
            }

            let out_data = OutputData {
                name: k.bold().to_string(),
                exp: access_token
                    .expires_at
                    .format(constants::LOCAL_DATETIME_FORMAT)
                    .to_string()
                    .dimmed()
                    .magenta()
                    .to_string(),
                acpl: access_token.acpl.join(", ").blue().bold().to_string(),
                status: match &state.active_token {
                    ActiveToken::Tag(tag) if *tag == *k => "[v]".dimmed().green().to_string(),
                    _ => "[ ]".dimmed().bright_black().to_string(),
                },
            };

            name_padding = name_padding.max(out_data.name.len());
            acpl_padding = acpl_padding.max(out_data.acpl.len());
            exp_padding = exp_padding.max(out_data.exp.len());

            output_tuples.push(out_data);
        }
    }

    _ = state.guard_mutate(|state| {
        for k in exp_keys.iter() {
            state.tokens.remove(k);
        }

        Ok(())
    });

    for out in output_tuples {
        let (name, exp, acpl, status) = (out.name, out.exp, out.acpl, out.status);

        println!(
            "{}",
            format!("{status} {name:<name_padding$} {exp:<exp_padding$} {acpl:<acpl_padding$}")
        );
    }
}

pub async fn handle_blacklist_token(tokens: &Vec<String>) {
    let ctx = AppContext {
        config: &CONFIG.try_lock().unwrap(),
        state: &STATE.try_lock().unwrap(),
    };

    api::tokens::blacklist_token(&ctx, &tokens)
        .await
        .expect("error occured while blacklisting token!");

    println!("{}", "Token blacklisted successfully!".to_string().bold());
}

pub async fn handle_generate(acpl: &Vec<String>, tag: Option<&str>, exp_input: &ExpInput) {
    if acpl.len() == 0 {
        println!(
            "{}",
            String::from("at least 1 ACPL is required to generate an access token!").bold()
        );
        return;
    }

    let ctx = AppContext {
        config: &CONFIG.try_lock().unwrap(),
        state: &STATE.try_lock().unwrap(),
    };
    let wd = ctx.config.get_wd();

    let expires_at: DateTime<Utc> = exp_input.get_expires_at();

    let res_data = api::tokens::generate_access_token(&ctx, &acpl, &expires_at)
        .await
        .expect("error occured while requesting API for a new access token!");
    let access_token_data: AccessToken = res_data
        .access_token
        .parse()
        .expect("access token parsing error!");

    let mut state = STATE.lock().unwrap();
    if let Some(tag) = tag {
        _ = state.guard_mutate(|s| {
            s.tokens
                .insert(tag.to_string(), res_data.access_token.clone());

            Ok(())
        });
    }

    println!("{} ", res_data.access_token.cyan().bold());
    println!(
        "{}",
        format!(
            "(expires at: {})",
            access_token_data
                .expires_at
                .format(constants::LOCAL_DATETIME_FORMAT)
        )
        .dimmed()
    );
    println!();

    println!(
        "example usage in URL: {}",
        utils::files::get_share_url(Some(&res_data.access_token), "FILE_ID", &ctx)
            .expect("unexpected error occured while generating base url!")
            .to_string()
            .blue()
    );
    println!();

    println!("Directory tree visible to the access token user:");
    let mut opts = PrintDirTreeOpts::get_default_opts();
    opts.cwd_dir_path = wd;
    println!("{}", res_data.dirtree.print_dir_tree(&opts));
}

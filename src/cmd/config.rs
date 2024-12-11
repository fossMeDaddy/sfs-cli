use clap::Parser;

use crate::{
    config::{LogLevel, CONFIG},
    shared_types::CliSubCmd,
};

#[derive(Parser)]
#[group(required = true, multiple = false)]
pub struct ConfigCommand {
    #[arg(long)]
    /// default log level is "chirpy", it can be annoying i totally get why you'd wa-
    set_log_level: Option<LogLevel>,
}

impl CliSubCmd for ConfigCommand {
    async fn run(&self) {
        let mut config = CONFIG.try_lock().unwrap();

        match self.set_log_level.clone() {
            Some(log_level) => config
                .set_log_level(log_level)
                .expect("error occured while writing to config file"),
            None => {}
        }
    }
}

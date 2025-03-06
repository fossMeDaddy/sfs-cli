use chrono::{DateTime, Duration, Local, Utc};
use clap::{Args, ValueEnum};

use crate::utils::str2x;

#[derive(Debug, Args)]
#[group(multiple = false)]
pub struct CmdExpiryParams {
    #[arg(long, value_parser = str2x::str2datetime)]
    /// provide an expiry datetime in your local timezone. format: YYYY-mm-dd HH:MM:SS
    expires_at: Option<DateTime<Local>>,

    #[arg(long, value_parser = str2x::str2duration)]
    /// provide an expiry duration. format: 1d2h3m4s, default: 30m
    ttl: Option<Duration>,
}

impl CmdExpiryParams {
    pub fn is_unset(&self) -> bool {
        self.expires_at.is_none() && self.ttl.is_none()
    }

    /// if nothing provided, 30 mins expiry duration is set
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

#[derive(Clone, ValueEnum)]
pub enum CmdVisibility {
    Public,
    Private,
}

use serde::{Deserialize, Serialize};


mod models;
mod tokens;
mod uploads;
mod usage;

pub use models::*;
pub use tokens::*;
pub use uploads::*;
pub use usage::*;

use crate::{
    config::{CliConfig},
    state::{PersistentState},
};

pub trait CliSubCmd {
    async fn run(&self);
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub message: String,
    pub data: Option<T>,
    pub error: Option<String>,
}

pub struct AppContext<'a> {
    pub config: &'a CliConfig,
    pub state: &'a PersistentState,
}

use serde::{Deserialize, Serialize};

mod cmd;
mod models;
mod tokens;
mod uploads;
mod usage;

pub use cmd::*;
pub use models::*;
pub use tokens::*;
pub use uploads::*;
pub use usage::*;

pub trait CliSubCmd {
    async fn run(&self);
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub message: String,
    pub data: Option<T>,
    pub error: Option<String>,
}

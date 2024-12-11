use serde::Deserialize;

mod api;
mod models;
mod tokens;

pub use api::*;
pub use models::*;
pub use tokens::*;

pub trait CliSubCmd {
    async fn run(&self);
}

#[derive(Deserialize)]
pub struct ApiResponse<T> {
    pub message: String,
    pub data: Option<T>,
    pub error: Option<String>,
}

use crate::app::App;

mod app;
pub mod auth;
mod db;
pub mod error;
pub mod models;
pub mod repository;
pub mod routes;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    App::start().await
}

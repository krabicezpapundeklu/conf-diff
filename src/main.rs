use anyhow::Result;

use crate::{repository::new_repository, server::start, service::new_service};

mod model;
mod repository;
mod server;
mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let repository = new_repository()?;
    let service = new_service(repository);

    start(service).await
}

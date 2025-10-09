use anyhow::Result;

use crate::{
    repository::new_repository,
    service::{Service, new_service},
};

mod model;
mod repository;
mod service;

fn main() -> Result<()> {
    let repository = new_repository()?;
    let service = new_service(repository);
    let environment_names = service.get_environment_names()?;

    println!("{environment_names:?}");

    let environment_config_diffs = service.get_environment_config_diffs("qa252", "qa253")?;

    println!("{environment_config_diffs:?}");

    Ok(())
}

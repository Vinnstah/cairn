use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use crate::{
    core::domain::config::{Config, Dataset},
    error::ServerError,
};

pub mod domain;
pub mod ports;
pub mod services;

pub fn build_dataset_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/nvidia_physical_dataset")
}

impl Dataset {
    pub async fn load_from_config() -> Result<Config, ServerError> {
        let mut config_str = File::open("../dataset.toml")?;
        let mut buffer = String::new();
        let _ = config_str.read_to_string(&mut buffer)?;
        let config: Config = toml::from_str(buffer.as_str())?;
        Ok(config)
    }
}

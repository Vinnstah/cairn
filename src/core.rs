use std::path::PathBuf;

pub mod domain;
pub mod ports;

pub fn build_dataset_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/nvidia_physical_dataset")
}

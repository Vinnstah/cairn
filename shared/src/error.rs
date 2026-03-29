use thiserror::Error;

#[derive(Error, Debug)]
pub enum CairnError {
    #[error("Query execution failed: '{0}'")]
    QueryFailed(String),

    #[error("Missing column in result: '{0}'")]
    MissingColumn(&'static str),

    #[error("Clip not found: '{0}'")]
    ClipNotFound(String),

    #[error("Failed to convert to type: '{0}'")]
    FailedToConvertToType(String),

    #[error("Invalid parameter '{param}': {reason}")]
    InvalidParam { param: &'static str, reason: String },

    #[error("Failed to read parquet file at {path}: {reason}")]
    ParquetRead { path: String, reason: String },

    #[error("Generic error: {reason}")]
    Generic { reason: String },
}

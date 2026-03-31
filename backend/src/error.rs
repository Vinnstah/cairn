use axum::{Json, http::StatusCode, response::IntoResponse};
use rerun::RecordingStreamError;
use serde_json::json;
use shared::error::CairnError;

// Add a newtype wrapper for CairnError to get around orphan rule and also to implement backend-specific traits for it without having to import the dependencies into shared.
pub struct ServerError(pub CairnError);

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match &self.0 {
            CairnError::QueryFailed(reason) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Query failed",
                format!("Query execution failed: {reason}"),
            ),
            CairnError::MissingColumn(column) => (
                StatusCode::BAD_REQUEST,
                "Missing column",
                format!("Missing column: {column}"),
            ),
            CairnError::ClipNotFound(id) => (
                StatusCode::NOT_FOUND,
                "Clip not found",
                format!("Clip not found: {id}"),
            ),
            CairnError::InvalidParam { param, reason } => (
                StatusCode::BAD_REQUEST,
                "Invalid Param",
                format!("Invalid Param: {param}: {reason}"),
            ),
            CairnError::ParquetRead { path, reason } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Parquet Read",
                format!("Parquet Read: {path} : {reason}"),
            ),
            CairnError::FailedToConvertToType(type_name) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to convert to type",
                format!("Failed to convert to type: {type_name}"),
            ),
            CairnError::Generic { reason } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Generic error",
                format!("Generic error: {reason}"),
            ),
            CairnError::StreamingError { reason } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Streaming error",
                format!("Streaming error: {reason}"),
            ),
        };
        (
            status,
            Json(json!({"error": {
                "code": code,
                "message": message
            }})),
        )
            .into_response()
    }
}

impl From<CairnError> for ServerError {
    fn from(e: CairnError) -> Self {
        ServerError(e)
    }
}

impl From<datafusion::error::DataFusionError> for ServerError {
    fn from(e: datafusion::error::DataFusionError) -> Self {
        ServerError(CairnError::QueryFailed(e.to_string()))
    }
}

impl From<ServerError> for CairnError {
    fn from(value: ServerError) -> Self {
        CairnError::Generic {
            reason: value.0.to_string(),
        }
    }
}

impl From<RecordingStreamError> for ServerError {
    fn from(e: RecordingStreamError) -> Self {
        ServerError(CairnError::StreamingError {
            reason: e.to_string(),
        })
    }
}

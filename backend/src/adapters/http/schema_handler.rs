use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};

use crate::startup::AppState;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/schema", get(schema_handler))
        .with_state(state)
}

pub async fn schema_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.querier.fetch_schema().await {
        Ok(columns) => Json(columns).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.error_msg).into_response(),
    }
}

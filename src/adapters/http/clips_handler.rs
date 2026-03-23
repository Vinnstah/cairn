/// Endpoint to search for specific events or conditions within the dataset. Returns a clip_id to be used for the other endpoints.
use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};

use crate::{
    core::{domain::model::ClipSearchParams, ports::inbound::data_query::DataQuery},
    startup::AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/clips/search", get(clips_search_handler))
        .with_state(state)
}

async fn clips_search_handler(
    State(state): State<AppState>,
    Query(params): Query<ClipSearchParams>,
) -> impl IntoResponse {
    match state.querier.fetch_clips_with_params(params).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

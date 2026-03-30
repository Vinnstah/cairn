//! Endpoint to search for specific events or conditions within the dataset. Returns a clip_id to be used for the other endpoints.

use crate::startup::AppState;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use log::info;
use shared::ClipSearchParams;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/clips/search", get(clips_search_handler))
        .route("/clips/replay", post(clips_replay_handler))
        .with_state(state)
}

async fn clips_search_handler(
    State(state): State<AppState>,
    Query(params): Query<ClipSearchParams>,
) -> impl IntoResponse {
    info!("received clips search request");
    match state.querier.fetch_clips_with_params(params).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => err.into_response(),
    }
}

async fn clips_replay_handler(
    State(state): State<AppState>,
    Json(params): Json<ClipSearchParams>,
) -> impl IntoResponse {
    info!("received clip replay request");
    match state.replayer.replay_clips(params).await {
        Ok(_) => (StatusCode::OK, Json("replaying clips in rerun")).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

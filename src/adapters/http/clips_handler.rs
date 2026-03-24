/// Endpoint to search for specific events or conditions within the dataset. Returns a clip_id to be used for the other endpoints.
use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use rerun::Points3D;

use crate::{
    core::{
        domain::model::ClipSearchParams,
        ports::{inbound::data_query::DataQuery, outbound::replay::Replay},
    },
    startup::AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/clips/search", get(clips_search_handler))
        .route("/clips/replay", get(clips_replay_handler))
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

async fn clips_replay_handler(
    State(state): State<AppState>,
    Query(params): Query<ClipSearchParams>,
) -> impl IntoResponse {
    match state.querier.fetch_clips_with_params(params).await {
        Ok(result) => {
            for clip_id in result {
                if clip_id.is_empty() {
                    continue;
                }
                let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("data/nvidia_physical_dataset/lidar.chunk_0000")
                    .join(clip_id.clone() + ".lidar_top_360fov.parquet");
                if !path.exists() {
                    println!(
                        "[warn] lidar file not found for clip {}, skipping",
                        clip_id.clone()
                    );
                    continue;
                }
                let mut point_clouds: Vec<Points3D> = Vec::with_capacity(20);

                // Don't loop here, the replay should accept a Vec of Point3D
                for i in 1..20 {
                    let point_cloud = state
                        .querier
                        .fetch_point_cloud(&clip_id, i)
                        .await
                        .expect("get pc");
                    point_clouds.push(Points3D::new(point_cloud));
                    state
                        .replayer
                        .replay_point_cloud(point_clouds[i - 1].clone())
                        .await;
                }
            }
            (StatusCode::OK, Json("result")).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

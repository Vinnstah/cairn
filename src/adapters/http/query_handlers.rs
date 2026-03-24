use std::{collections::HashMap, path::PathBuf};

use axum::{
    Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use rerun::AssetVideo;

use crate::{
    core::ports::{inbound::data_query::DataQuery, outbound::replay::Replay},
    startup::AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/timespan", get(fetch_timespan_handler))
        .with_state(state)
}

#[axum::debug_handler]
pub async fn fetch_timespan_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    match state
        .querier
        .fetch_selected_time(crate::core::domain::model::Timespan {
            start: params["start"]
                .parse::<u64>()
                .expect("parse u64 from string"),
            end: params["end"].parse::<u64>().expect("parse u64 from string"),
        })
        .await
        .map_err(|err| err)
    {
        Ok(clip_id) => {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/nvidia_physical_dataset/camera_front_wide_120fov.chunk_0000")
                .join(clip_id + ".camera_front_wide_120fov.mp4");
            let video_asset = AssetVideo::from_file_path(path).expect("construct video asset");
            let _ = state.replayer.visualize_video(video_asset).await;
            Ok(())
        }
        Err(err) => {
            println!("{:#?}", err);
            Err(())
        }
    }
}

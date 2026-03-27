use std::sync::Arc;

use datafusion::prelude::SessionContext;
use log::info;

use crate::{
    adapters::http,
    core::{
        ports::{
            inbound::{data_query::DataQuery, replay::Replay},
            outbound::{data_store::DataStore, scene_logger::SceneLogger},
        },
        services::{data_query_service::DataQueryService, replay_service::ReplayService},
    },
};

pub async fn start() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let router = axum::Router::<()>::new();
    let querier_repo = Arc::new(SessionContext::new());
    let replayer_repo = rerun::RecordingStreamBuilder::new("replayer_repo")
        .spawn()
        .expect("create recording_stream");
    let app_state = AppState::new(querier_repo, Arc::new(replayer_repo));
    let _ = app_state.querier.register_tables().await;
    let app = router
        .merge(http::health_handlers::routes())
        .merge(http::clips_handler::routes(app_state.clone()))
        .merge(http::schema_handler::routes(app_state.clone()));
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("serving traffic on 0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
pub struct AppState {
    pub querier: Arc<dyn DataQuery + Send + Sync>,
    pub replayer: Arc<dyn Replay + Send + Sync>,
}

impl AppState {
    pub fn new(
        data_store: Arc<dyn DataStore + Send + Sync>,
        logger: Arc<dyn SceneLogger + Send + Sync>,
    ) -> Self {
        let data_query_service = Arc::new(DataQueryService::new(data_store));
        let replay_service = Arc::new(ReplayService::new(logger, data_query_service.clone()));
        Self {
            querier: data_query_service,
            replayer: replay_service,
        }
    }
}

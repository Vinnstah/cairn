use std::sync::Arc;

use datafusion::prelude::SessionContext;
use log::info;

use crate::{
    adapters::http,
    core::ports::{
        data_query_service::DataQueryService,
        inbound::data_query::DataQuery,
        outbound::{data_store::DataStore, replay::Replay},
        replay_service::ReplayService,
    },
};

pub async fn start() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let router = axum::Router::<()>::new();
    let querier_repo = Arc::new(SessionContext::new());
    let replayer_repo = rerun::RecordingStreamBuilder::new("replayer_repo")
        .spawn()
        .expect("create recording_stream");
    let app_state = AppState::new(querier_repo, Arc::new(replayer_repo));
    let _ = app_state.querier.register_tables().await;
    let app = router
        .merge(http::health_handlers::routes())
        .merge(http::clips_handler::routes(app_state.clone()));
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("serving traffic on 0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
pub struct AppState {
    pub querier: DataQueryService,
    pub replayer: ReplayService,
}

impl AppState {
    pub fn new(
        querier: Arc<dyn DataStore + Send + Sync>,
        replayer: Arc<dyn Replay + Send + Sync>,
    ) -> Self {
        let data_query_service = DataQueryService::new(querier);
        let replay_service = ReplayService::new(replayer);
        Self {
            querier: data_query_service,
            replayer: replay_service,
        }
    }
}

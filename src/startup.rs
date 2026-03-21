use std::sync::Arc;

use datafusion::prelude::SessionContext;

use crate::{
    adapters::http,
    core::ports::{
        data_query_service::DataQueryService, outbound::data_repository::DataQueryRepository,
    },
};

pub async fn start() {
    let router = axum::Router::<()>::new();
    let repo = Arc::new(SessionContext::new());
    let _ = repo.register_parquets();
    let service = Arc::new(DataQueryService::new(repo));
    let app = router
        .merge(http::query_handlers::routes(service))
        .merge(http::health_handlers::routes());
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

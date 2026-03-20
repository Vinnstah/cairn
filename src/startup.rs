use axum::routing::get;

use crate::adapters::http;

pub async fn start() {
    let router = axum::Router::<()>::new();
    // let query_service = DataQuery

    let app = router
        .route("/", get(|| async { "Hello, World!" }))
        .merge(http::health_handlers::routes());
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

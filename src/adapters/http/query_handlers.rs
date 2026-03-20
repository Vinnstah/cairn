// use axum::routing::get;

// impl<S> RouteDelegator for axum::Router<S>
// where
//     S: Clone + Send + Sync + 'static,
// {
//     async fn serve(&self) -> () {
//         let app = axum::Router::new().route("/", get(|| async { "Hello, World!" }));
//         // run our app with hyper, listening globally on port 3000
//         let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//         axum::serve(listener, app).await.unwrap();
//     }

//     async fn handle_get_timesequence(
//         &self,
//         querier: impl Querier,
//         timespan: crate::core::domain::model::Timespan,
//     ) -> Result<String, ()> {
//         self.route(
//             "/",
//             get(|| async { querier.query_selected_time(timespan).await }),
//         );
//         Ok(())
//     }
// }

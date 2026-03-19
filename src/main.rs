use datafusion::prelude::SessionContext;

use crate::{
    app::service::Service,
    domain::{
        model::Timespan,
        port::{Querier, RouteDelegator},
    },
};
mod app;
mod domain;
mod inbound;
mod outbound;

#[tokio::main]
async fn main() {
    let sc = SessionContext::new();
    let router = axum::Router::<()>::new();
    let service = Service::new(sc.into(), router.into());
    let res = service
        .querier
        .query_selected_time(Timespan::new(1700000220050000, 1700000220210000))
        .await;
    println!("{:#?}", res.unwrap());
    service.router.serve().await;
}

use datafusion::prelude::SessionContext;

use crate::{
    app::service::Service,
    domain::{model::Timespan, port::Querier},
};
mod app;
mod domain;
mod outbound;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sc = SessionContext::new();
    let state = Service::new(sc.into());
    let res = state
        .querier
        .query_selected_time(Timespan::new(1700000220050000, 1700000220210000))
        .await;
    println!("{:#?}", res.unwrap());
    Ok(())
}

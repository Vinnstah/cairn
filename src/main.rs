use datafusion::prelude::SessionContext;

use crate::{
    client::AppState,
    querier::querier::{Querier, Timespan},
};
mod client;
mod querier;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sc = SessionContext::new();
    let state = AppState::new(sc.into());
    let _ = state
        .querier
        .query_selected_time(Timespan::new(1700000220050000, 1700000220210000))
        .await;
    Ok(())
}

use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};

use crate::core::ports::inbound::data_query::DataQuery;

pub fn routes(service: Arc<dyn DataQuery>) -> Router {
    Router::new()
        .route("/timespan", get(fetch_timespan_handler))
        .with_state(service)
}

#[axum::debug_handler]
pub async fn fetch_timespan_handler(
    State(service): State<Arc<dyn DataQuery>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    match service
        .fetch_selected_time(crate::core::domain::model::Timespan {
            start: params["start"]
                .parse::<u64>()
                .expect("parse u64 from string"),
            end: params["end"].parse::<u64>().expect("parse u64 from string"),
        })
        .await
        .map_err(|err| println!("{ :#? }", err))
    {
        Ok(result) => println!("{}", result),
        Err(_) => todo!(),
    }
}

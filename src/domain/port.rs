use crate::domain::model::{DataError, Timespan};

pub trait Querier {
    async fn query_selected_time(&self, timespan: Timespan) -> Result<String, DataError>;
}

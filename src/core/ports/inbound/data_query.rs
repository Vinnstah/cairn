use crate::core::domain::model::{DataError, Timespan};

#[async_trait::async_trait]
pub trait DataQuery: Send + Sync {
    async fn fetch_selected_time(&self, timespan: Timespan) -> Result<String, DataError>;
}

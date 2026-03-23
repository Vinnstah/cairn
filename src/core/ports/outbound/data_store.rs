use crate::core::domain::model::{ClipSearchParams, DataError, Timespan};

#[async_trait::async_trait]
pub trait DataStore {
    async fn query_selected_timespan(
        &self,
        timespan: Timespan,
    ) -> anyhow::Result<String, DataError>;
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn query_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>>;
}

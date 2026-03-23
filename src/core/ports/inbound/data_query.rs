use crate::core::domain::model::{ClipSearchParams, DataError, Timespan};

#[async_trait::async_trait]
pub trait DataQuery: Send + Sync {
    async fn fetch_selected_time(&self, timespan: Timespan) -> Result<String, DataError>;
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn search_clips_dataset(&self, params: ClipSearchParams) -> anyhow::Result<Vec<String>>;
}

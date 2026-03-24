use crate::core::domain::model::{ClipSearchParams, DataError, Timespan};

#[async_trait::async_trait]
pub trait DataQuery: Send + Sync {
    async fn fetch_selected_time(&self, timespan: Timespan) -> Result<String, DataError>;
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn fetch_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>>;
    async fn fetch_point_cloud(
        &self,
        clip_id: &str,
        spin_index: usize,
    ) -> Result<Vec<[f32; 3]>, DataError>;
}

use crate::core::domain::model::{ClipSearchParams, DataError};

#[async_trait::async_trait]
pub trait DataStore {
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn query_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>>;
    async fn query_point_cloud(
        &self,
        clip_id: &str,
        spin_index: usize,
    ) -> Result<Vec<[f32; 3]>, DataError>;
}

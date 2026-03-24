use crate::core::domain::model::{ClipSearchParams, DataError, PointCloud};

#[async_trait::async_trait]
pub trait DataStore {
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn query_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>>;
    async fn query_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, DataError>;
}

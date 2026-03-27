use shared::ColumnInfo;

use crate::core::domain::model::{ClipSearchParams, DataError, EgoMotion, PointCloud};

#[async_trait::async_trait]
pub trait DataQuery: Send + Sync {
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn fetch_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>>;
    async fn fetch_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, DataError>;
    async fn fetch_ego_motion(&self, clip_id: &str) -> Result<Vec<EgoMotion>, DataError>;
    async fn fetch_schema(&self) -> Result<Vec<ColumnInfo>, DataError>;
}

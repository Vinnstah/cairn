use shared::ColumnInfo;

use crate::core::domain::model::{ClipSearchParams, DataError, EgoMotion, PointCloud};

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
    async fn query_ego_motion(&self, clip_id: &str) -> anyhow::Result<Vec<EgoMotion>, DataError>;
    async fn query_schema(&self) -> Result<Vec<ColumnInfo>, DataError>;
}

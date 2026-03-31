use shared::{ClipSearchParams, SchemaResponse};

use crate::{
    core::domain::model::{BoundingBox, EgoMotion, PointCloud},
    error::ServerError,
};

#[async_trait::async_trait]
pub trait DataQuery: Send + Sync {
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn fetch_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> Result<Vec<String>, ServerError>;
    async fn fetch_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, ServerError>;
    async fn fetch_ego_motion(&self, clip_id: &str) -> Result<Vec<EgoMotion>, ServerError>;
    async fn fetch_schema(&self) -> Result<SchemaResponse, ServerError>;
    async fn fetch_bounding_boxes(&self, clip_id: &str) -> Result<Vec<BoundingBox>, ServerError>;
}

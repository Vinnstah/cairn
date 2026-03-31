use shared::{ClipSearchParams, SchemaResponse};

use crate::{
    core::domain::model::{BoundingBox, EgoMotion, PointCloud},
    error::ServerError,
};

#[async_trait::async_trait]
pub trait DataStore {
    async fn register_tables(&self) -> anyhow::Result<()>;
    async fn query_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> Result<Vec<String>, ServerError>;
    async fn query_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, ServerError>;
    async fn query_ego_motion(&self, clip_id: &str) -> anyhow::Result<Vec<EgoMotion>, ServerError>;
    async fn query_schema(&self) -> Result<SchemaResponse, ServerError>;
    async fn query_bounding_boxes(&self, clip_id: &str) -> Result<Vec<BoundingBox>, ServerError>;
}

use crate::{
    core::domain::model::{BoundingBox, EgoMotion, PointCloud},
    error::ServerError,
};

/// Trait that handles replaying into external replay-platform
#[async_trait::async_trait]
pub trait SceneLogger {
    async fn replay_video(&self, video: rerun::archetypes::AssetVideo) -> Result<(), ServerError>;
    async fn replay_point_clouds(
        &self,
        point_cloud: Vec<PointCloud>,
        time_offset_us: i64,
    ) -> Result<(), ServerError>;
    async fn replay_ego_motion(
        &self,
        ego_motion: Vec<EgoMotion>,
        time_offset_us: i64,
    ) -> Result<(), ServerError>;
    async fn replay_bounding_boxes(
        &self,
        boxes: Vec<BoundingBox>,
        time_offset_us: i64,
    ) -> Result<(), ServerError>;
}

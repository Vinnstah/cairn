use crate::{
    core::domain::model::{BoundingBox, EgoMotion, PointCloud},
    error::ServerError,
};

#[async_trait::async_trait]
pub trait SceneLogger {
    async fn visualize_video(
        &self,
        video: rerun::archetypes::AssetVideo,
    ) -> Result<(), ServerError>;
    async fn replay_point_clouds(&self, point_cloud: Vec<PointCloud>) -> Result<(), ServerError>;
    async fn replay_ego_motion(&self, ego_motion: Vec<EgoMotion>) -> Result<(), ServerError>;
    async fn replay_bounding_boxes(&self, boxes: Vec<BoundingBox>) -> Result<(), ServerError>;
}

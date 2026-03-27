use crate::core::domain::model::{EgoMotion, PointCloud};

#[async_trait::async_trait]
pub trait SceneLogger {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()>;
    async fn replay_point_clouds(&self, point_cloud: Vec<PointCloud>) -> anyhow::Result<()>;
    async fn replay_ego_motion(&self, ego_motion: Vec<EgoMotion>) -> anyhow::Result<()>;
}

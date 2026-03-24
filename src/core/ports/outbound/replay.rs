use crate::core::domain::model::PointCloud;

#[async_trait::async_trait]
pub trait Replay {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()>;
    async fn replay_point_clouds(&self, point_cloud: Vec<PointCloud>) -> anyhow::Result<()>;
}

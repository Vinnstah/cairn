use rerun::Points3D;

#[async_trait::async_trait]
pub trait Replay {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()>;
    async fn replay_point_cloud(&self, point_cloud: Points3D) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait VisualizerRepository {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait Visualize {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()>;
}

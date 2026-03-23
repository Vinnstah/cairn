#[async_trait::async_trait]
pub trait Replay {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()>;
}

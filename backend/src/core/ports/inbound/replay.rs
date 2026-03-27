use crate::core::domain::model::ClipSearchParams;

#[async_trait::async_trait]
pub trait Replay: Send + Sync {
    async fn replay_clips(&self, params: ClipSearchParams) -> anyhow::Result<()>;
}

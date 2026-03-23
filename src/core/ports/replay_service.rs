use std::sync::Arc;

use crate::core::ports::outbound::replay::Replay;

#[derive(Clone)]
pub struct ReplayService {
    repo: Arc<dyn Replay + Send + Sync>,
}

impl ReplayService {
    pub fn new(repo: Arc<dyn Replay + Send + Sync>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl Replay for ReplayService {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()> {
        // TODO: Add logging, metrics and telemetry here
        self.repo.visualize_video(video).await
    }
}

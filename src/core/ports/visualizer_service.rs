use std::sync::Arc;

use crate::core::ports::outbound::visualizer_repository::{Visualize, VisualizerRepository};

#[derive(Clone)]
pub struct VisualizerService {
    repo: Arc<dyn VisualizerRepository + Send + Sync>,
}

impl VisualizerService {
    pub fn new(repo: Arc<dyn VisualizerRepository + Send + Sync>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl Visualize for VisualizerService {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()> {
        // TODO: Add logging, metrics and telemetry here
        self.repo.visualize_video(video).await
    }
}

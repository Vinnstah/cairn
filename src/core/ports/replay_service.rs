use crate::core::{domain::model::PointCloud, ports::outbound::replay::Replay};
use std::sync::Arc;

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

    async fn replay_point_clouds(&self, point_clouds: Vec<PointCloud>) -> anyhow::Result<()> {
        self.repo.replay_point_clouds(point_clouds).await
    }
}

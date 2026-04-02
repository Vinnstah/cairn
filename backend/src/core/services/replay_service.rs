use log::info;
use shared::ClipSearchParams;

use crate::{
    core::ports::{
        inbound::{data_query::DataQuery, replay::Replay},
        outbound::scene_logger::SceneLogger,
    },
    error::ServerError,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct ReplayService {
    query: Arc<dyn DataQuery + Send + Sync>,
    logger: Arc<dyn SceneLogger + Send + Sync>,
}

impl ReplayService {
    pub fn new(
        logger: Arc<dyn SceneLogger + Send + Sync>,
        query: Arc<dyn DataQuery + Send + Sync>,
    ) -> Self {
        Self { logger, query }
    }
}

#[async_trait::async_trait]
impl Replay for ReplayService {
    /// Load and replay clips in Rerun
    async fn replay_clips(&self, params: ClipSearchParams) -> Result<(), ServerError> {
        let clip_ids = self.query.fetch_clips_with_params(params).await?;

        for clip_id in &clip_ids {
            let t = std::time::Instant::now();
            let point_clouds = self.query.fetch_point_clouds(clip_id, 200).await?;
            info!("fetch_point_clouds: {}ms", t.elapsed().as_millis());

            let t = std::time::Instant::now();
            let ego_motion = self.query.fetch_ego_motion(clip_id).await?;
            info!("fetch_ego_motion: {}ms", t.elapsed().as_millis());

            let t = std::time::Instant::now();
            let bounding_boxes = self.query.fetch_bounding_boxes(clip_id).await?;
            info!("fetch_bounding_boxes: {}ms", t.elapsed().as_millis());

            let t = std::time::Instant::now();
            self.logger.replay_point_clouds(point_clouds).await?;
            info!("replay_point_clouds logging: {}ms", t.elapsed().as_millis());

            let t = std::time::Instant::now();
            self.logger.replay_ego_motion(ego_motion).await?;
            info!("replay_ego_motion logging: {}ms", t.elapsed().as_millis());

            let t = std::time::Instant::now();
            self.logger.replay_bounding_boxes(bounding_boxes).await?;
            info!(
                "replay_bounding_boxes logging: {}ms",
                t.elapsed().as_millis()
            );
            info!("rerun logging: {}ms", t.elapsed().as_millis());
        }
        Ok(())
    }
}

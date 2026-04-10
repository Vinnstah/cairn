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

        let mut time_offset_us: i64 = 0;

        for clip_id in &clip_ids {
            let t = std::time::Instant::now();
            let point_clouds = self.query.fetch_point_clouds(clip_id, 200).await?;
            let ego_motion = self.query.fetch_ego_motion(clip_id).await?;
            let bounding_boxes = self.query.fetch_bounding_boxes(clip_id).await?;

            // Find the time range of this clip from ego motion
            let min_ts = bounding_boxes
                .iter()
                .map(|e| e.timestamp_us)
                .min()
                .unwrap_or(0);
            let max_ts = bounding_boxes
                .iter()
                .map(|e| e.timestamp_us)
                .max()
                .unwrap_or(0);
            let clip_duration_us = max_ts - min_ts;

            self.logger
                .replay_point_clouds(point_clouds, time_offset_us)
                .await?;
            self.logger
                .replay_ego_motion(ego_motion, time_offset_us)
                .await?;
            self.logger
                .replay_bounding_boxes(bounding_boxes, time_offset_us)
                .await?;

            info!("replay clip {}: {}ms", clip_id, t.elapsed().as_millis());
            let gap_us = 500_000; // 0.5 second gap between clips
            time_offset_us += clip_duration_us + gap_us;
        }
        Ok(())
    }
}

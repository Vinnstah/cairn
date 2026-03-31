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
    async fn replay_clips(&self, params: ClipSearchParams) -> Result<(), ServerError> {
        let clip_ids = self
            .query
            .fetch_clips_with_params(params)
            .await
            .map_err(|err| err.0)?;
        info!("replay clips, {:#?}", clip_ids);

        for clip_id in clip_ids {
            info!("replaying clip {}", clip_id);

            let point_clouds = self.query.fetch_point_clouds(&clip_id, 100).await?;
            info!("fetched {} point clouds", point_clouds.len());

            let ego_motion = self.query.fetch_ego_motion(&clip_id).await?;
            info!("fetched {} ego motion samples", ego_motion.len());

            let bounding_boxes = self.query.fetch_bounding_boxes(&clip_id).await?;
            info!("fetched {} bounding boxes", bounding_boxes.len());

            self.logger.replay_ego_motion(ego_motion).await?;
            self.logger.replay_point_clouds(point_clouds).await?;
            self.logger.replay_bounding_boxes(bounding_boxes).await?;

            info!("done replaying clip {}", clip_id);
        }
        Ok(())
    }
}

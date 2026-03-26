use log::{error, info, warn};

use crate::core::{
    domain::model::ClipSearchParams,
    ports::{
        inbound::{data_query::DataQuery, replay::Replay},
        outbound::scene_logger::SceneLogger,
    },
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
    async fn replay_clips(&self, params: ClipSearchParams) -> anyhow::Result<()> {
        let clip_ids = self.query.fetch_clips_with_params(params).await?;
        info!("replay clips, {:#?}", clip_ids);
        for clip_id in clip_ids {
            match self.query.fetch_point_clouds(&clip_id, 50).await {
                Ok(point_clouds) => {
                    self.logger.replay_point_clouds(point_clouds).await?;
                }
                Err(e) => {
                    warn!("skipping clip {}: {}", clip_id, e.error_msg);
                }
            }
            match self.query.fetch_ego_motion(&clip_id).await {
                Ok(ego_motion) => self.logger.replay_ego_motion(ego_motion).await?,
                Err(err) => error!("{}", err.error_msg),
            }
        }
        Ok(())
    }
}

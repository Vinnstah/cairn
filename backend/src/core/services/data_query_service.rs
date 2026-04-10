use std::sync::Arc;

use crate::{
    core::{
        domain::{
            config::Config,
            model::{BoundingBox, EgoMotion, PointCloud},
        },
        ports::{inbound::data_query::DataQuery, outbound::data_store::DataStore},
    },
    error::ServerError,
};
use shared::{ClipSearchParams, SchemaResponse};

#[derive(Clone)]
pub struct DataQueryService {
    config: Config,
    data_store: Arc<dyn DataStore + Send + Sync>,
}

impl DataQueryService {
    pub fn new(config: Config, data_store: Arc<dyn DataStore + Send + Sync>) -> Self {
        Self { config, data_store }
    }
}

#[async_trait::async_trait]
impl DataQuery for DataQueryService {
    async fn register_tables(&self) -> Result<(), ServerError> {
        self.data_store
            .register_tables(self.config.datasets.clone())
            .await
    }

    async fn fetch_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> Result<Vec<String>, ServerError> {
        self.data_store.query_clips_with_params(params).await
    }

    async fn fetch_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, ServerError> {
        self.data_store.query_point_clouds(clip_id, num_spins).await
    }

    async fn fetch_ego_motion(&self, clip_id: &str) -> Result<Vec<EgoMotion>, ServerError> {
        self.data_store.query_ego_motion(clip_id).await
    }

    async fn fetch_schema(&self) -> Result<SchemaResponse, ServerError> {
        self.data_store.query_schema(&self.config).await
    }

    async fn fetch_bounding_boxes(&self, clip_id: &str) -> Result<Vec<BoundingBox>, ServerError> {
        self.data_store.query_bounding_boxes(clip_id).await
    }
}

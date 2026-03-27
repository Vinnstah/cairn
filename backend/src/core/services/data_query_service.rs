use std::sync::Arc;

use shared::ColumnInfo;

use crate::core::{
    domain::model::{ClipSearchParams, DataError, EgoMotion, PointCloud},
    ports::{inbound::data_query::DataQuery, outbound::data_store::DataStore},
};

#[derive(Clone)]
pub struct DataQueryService {
    data_store: Arc<dyn DataStore + Send + Sync>,
}

impl DataQueryService {
    pub fn new(data_store: Arc<dyn DataStore + Send + Sync>) -> Self {
        Self { data_store }
    }
}

#[async_trait::async_trait]
impl DataQuery for DataQueryService {
    async fn register_tables(&self) -> anyhow::Result<()> {
        self.data_store.register_tables().await
    }

    async fn fetch_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>> {
        let result = self.data_store.query_clips_with_params(params).await;
        Ok(result.expect("fetch clips with params"))
    }

    async fn fetch_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, DataError> {
        self.data_store.query_point_clouds(clip_id, num_spins).await
    }

    async fn fetch_ego_motion(&self, clip_id: &str) -> Result<Vec<EgoMotion>, DataError> {
        self.data_store.query_ego_motion(clip_id).await
    }

    async fn fetch_schema(&self) -> Result<Vec<ColumnInfo>, DataError> {
        self.data_store.query_schema().await
    }
}

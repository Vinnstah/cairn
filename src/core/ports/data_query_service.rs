use std::sync::Arc;

use crate::core::{
    domain::model::{ClipSearchParams, DataError, PointCloud},
    ports::{inbound::data_query::DataQuery, outbound::data_store::DataStore},
};

#[derive(Clone)]
pub struct DataQueryService {
    repo: Arc<dyn DataStore + Send + Sync>,
}

impl DataQueryService {
    pub fn new(repo: Arc<dyn DataStore + Send + Sync>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl DataQuery for DataQueryService {
    async fn register_tables(&self) -> anyhow::Result<()> {
        self.repo.register_tables().await
    }

    async fn fetch_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>> {
        let result = self.repo.query_clips_with_params(params).await;
        Ok(result.expect("fetch clips with params"))
    }

    async fn fetch_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, DataError> {
        self.repo.query_point_clouds(clip_id, num_spins).await
    }
}

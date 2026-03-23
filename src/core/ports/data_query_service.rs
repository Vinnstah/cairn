use std::sync::Arc;

use crate::core::{
    domain::model::ClipSearchParams,
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
    async fn fetch_selected_time(
        &self,
        timespan: crate::core::domain::model::Timespan,
    ) -> Result<String, crate::core::domain::model::DataError> {
        // TODO: Add logging, metrics and telemetry here
        self.repo.query_selected_timespan(timespan).await
    }

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
}

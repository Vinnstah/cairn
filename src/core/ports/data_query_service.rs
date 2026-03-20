use std::sync::Arc;

use crate::core::ports::{
    inbound::data_query::DataQuery, outbound::data_repository::DataQueryRepository,
};

pub struct DataQueryService {
    repo: Arc<dyn DataQueryRepository + Send + Sync>,
}

impl DataQueryService {
    pub fn new(repo: Arc<dyn DataQueryRepository + Send + Sync>) -> Self {
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
}

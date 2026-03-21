use crate::core::domain::model::{DataError, Timespan};

#[async_trait::async_trait]
pub trait DataQueryRepository {
    async fn query_selected_timespan(
        &self,
        timespan: Timespan,
    ) -> anyhow::Result<String, DataError>;
    async fn register_parquets(&self) -> anyhow::Result<()>;
}

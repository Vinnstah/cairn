use std::env;

use datafusion::{
    arrow::util::pretty::pretty_format_batches,
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};

use crate::core::{
    domain::model::{DataError, Timespan},
    ports::outbound::data_repository::DataQueryRepository,
};

#[async_trait::async_trait]
impl DataQueryRepository for SessionContext {
    async fn query_selected_timespan(&self, timespan: Timespan) -> Result<String, DataError> {
        let home = env::var("HOME").expect("HOME not set");
        let base = format!("{}/Developer/Rust/cairn/data/nvidia_physical_dataset", home);

        self.register_parquet(
            "feature_presence",
            &format!("{}/feature_presence/", base),
            ParquetReadOptions::default(),
        )
        .await?;
        let _ = self
            .register_parquet(
                "data_collection",
                "~/Developer/Rust/cairn/data/nvidia_physical_dataset/",
                ParquetReadOptions::default(),
            )
            .await;
        let df = self
            .sql(format!("SELECT clip_id FROM feature_presence ").as_str())
            // .sql(format!("SELECT clip_id FROM feature_presence WHERE timestamp >= {} and timestamp <= {} ORDER BY timestamp", timespan.start, timespan.end).as_str())
            .await?;
        df.clone()
            .collect()
            .await
            .map_err(|err| err.into())
            .map(|res| {
                pretty_format_batches(&res)
                    .expect("pretty format RecordBatch")
                    .to_string()
            })
    }
}

impl From<DataFusionError> for DataError {
    fn from(value: DataFusionError) -> Self {
        DataError::new(value.message().to_string())
    }
}

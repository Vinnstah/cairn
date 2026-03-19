use std::env;

use datafusion::{
    arrow::util::pretty::pretty_format_batches,
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};

use crate::domain::{
    model::{DataError, Timespan},
    port::Querier,
};

impl Querier for SessionContext {
    async fn query_selected_time(&self, timespan: Timespan) -> Result<String, DataError> {
        let home = env::var("HOME").expect("HOME not set");
        let base = format!("{}/Developer/Rust/cairn/data/synthetic_data", home);

        self.register_parquet(
            "ego_motion",
            &format!("{}/ego_motion/", base),
            ParquetReadOptions::default(),
        )
        .await?;
        let _ = self
            .register_parquet(
                "metadata",
                "~/Developer/Rust/cairn/data/synthetic_data/metadata/",
                ParquetReadOptions::default(),
            )
            .await;
        let df = self
            .sql(format!("SELECT * FROM ego_motion WHERE timestamp >= {} and timestamp <= {} ORDER BY timestamp", timespan.start, timespan.end).as_str())
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

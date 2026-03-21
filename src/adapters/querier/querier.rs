use std::{env, path::PathBuf};

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
    async fn query_selected_timespan(
        &self,
        timespan: Timespan,
    ) -> anyhow::Result<String, DataError> {
        let df = self
            .sql(format!("SELECT * FROM feature_presence LIMIT 10").as_str())
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

    async fn register_parquets(&self) -> anyhow::Result<()> {
        // The dataset is saved locally at ./data/nvidia_physical_dataset
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/nvidia_physical_dataset");

        self.register_parquet(
            "ego_motion",
            base.join("egomotion.chunk_*/*.parquet").to_str().unwrap(),
            ParquetReadOptions::default(),
        )
        .await?;

        self.register_parquet(
            "data_collection",
            base.join("metadata/data_collection.parquet")
                .to_str()
                .unwrap(),
            ParquetReadOptions::default(),
        )
        .await?;

        self.register_parquet(
            "feature_presence",
            base.join("metadata/feature_presence.parquet")
                .to_str()
                .unwrap(),
            ParquetReadOptions::default(),
        )
        .await?;

        println!("Registered parquets from {}", base.display());
        Ok(())
    }
}

impl From<DataFusionError> for DataError {
    fn from(value: DataFusionError) -> Self {
        DataError::new(value.message().to_string())
    }
}

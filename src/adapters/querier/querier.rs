use std::{
    env,
    path::{Path, PathBuf},
};

use datafusion::{
    arrow::{self, array::Array},
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};

use crate::{
    adapters::querier::register_helper::register_with_clip_id,
    core::{
        domain::model::{DataError, Timespan},
        ports::outbound::data_store::DataStore,
    },
};
#[async_trait::async_trait]
impl DataStore for SessionContext {
    async fn query_selected_timespan(&self, timespan: Timespan) -> Result<String, DataError> {
        let df = self
            .sql(
                "SELECT e.clip_id
                 FROM ego_motion e
                 JOIN camera_timestamps c
                     ON e.clip_id = c.clip_id
                     AND ABS(e.timestamp - c.timestamp) < 1000
                 LIMIT 1",
            )
            .await?;

        let batches = df.collect().await.map_err(DataError::from)?;

        batches
            .first()
            .and_then(|batch| {
                batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<arrow::array::StringArray>()
            })
            .and_then(|arr| {
                if arr.is_empty() {
                    None
                } else {
                    Some(arr.value(0).to_string())
                }
            })
            .ok_or_else(|| DataError {
                error_msg: "No clips found for given timespan".into(),
            })
    }

    async fn register_tables(&self) -> anyhow::Result<()> {
        // The dataset is saved locally at ./data/nvidia_physical_dataset
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/nvidia_physical_dataset");
        register_with_clip_id(
            self,
            &base.join("egomotion.chunk_0000"),
            ".egomotion.parquet",
            "ego_motion",
        )
        .await?;

        register_with_clip_id(
            self,
            &base.join("camera_front_wide_120fov.chunk_0000"),
            ".camera_front_wide_120fov.timestamps.parquet",
            "camera_timestamps",
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

        self.register_parquet(
            "camera_timestamps",
            base.join("camera_front_wide_120fov.chunk_0000/*.timestamps.parquet")
                .to_str()
                .unwrap(),
            ParquetReadOptions::default(),
        )
        .await?;

        self.register_parquet(
            "lidar",
            base.join("lidar.chunk_0000/*.parquet").to_str().unwrap(),
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

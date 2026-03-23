use std::{env, path::PathBuf};

use datafusion::{
    arrow::{
        self,
        array::{Array, StringArray},
    },
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};

use crate::{
    adapters::querier::helpers::{build_search_query, register_with_clip_id},
    core::{
        domain::model::{ClipSearchParams, DataError, Timespan},
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

    async fn query_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> anyhow::Result<Vec<String>> {
        let df = self.sql(build_search_query(params).as_str()).await?;
        let batches = df.collect().await?;
        let mut result = vec![String::new()];
        for batch in batches {
            let clips_id = batch
                .column_by_name("clip_id")
                .expect("msg")
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("msg");
            result.push(clips_id.value(0).to_string())
        }
        println!("{:#?}", result);
        Ok(result)
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

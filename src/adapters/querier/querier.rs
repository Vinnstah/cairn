use std::{
    env,
    path::{Path, PathBuf},
};

use datafusion::{
    arrow::{self, array::Array},
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

        println!("Registered parquets from {}", base.display());
        Ok(())
    }
}

impl From<DataFusionError> for DataError {
    fn from(value: DataFusionError) -> Self {
        DataError::new(value.message().to_string())
    }
}

async fn register_with_clip_id(
    ctx: &SessionContext,
    dir: &Path,
    file_suffix: &str, // e.g. ".egomotion.parquet"
    table_name: &str,
) -> anyhow::Result<()> {
    let mut views = vec![];

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.ends_with(file_suffix))
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let filename = entry.file_name();
        let filename = filename.to_str().unwrap();
        let clip_id = filename.strip_suffix(file_suffix).unwrap();

        let alias = format!("{}_{}", table_name, i);
        ctx.register_parquet(
            &alias,
            path.to_str().unwrap(),
            ParquetReadOptions::default(),
        )
        .await?;

        views.push(format!("SELECT '{}' AS clip_id, * FROM {}", clip_id, alias));
    }

    if views.is_empty() {
        anyhow::bail!(
            "No files found in {} with suffix {}",
            dir.display(),
            file_suffix
        );
    }

    let sql = format!(
        "CREATE VIEW {} AS {}",
        table_name,
        views.join(" UNION ALL ")
    );
    ctx.sql(&sql).await?.collect().await?;

    println!("[{}] registered {} clips", table_name, views.len());
    Ok(())
}

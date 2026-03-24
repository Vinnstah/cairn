use datafusion::{
    arrow::{
        self,
        array::{Array, RecordBatch, StringArray},
    },
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};
use draco_rs::prelude::ffi::draco::GeometryAttribute_Type;
use draco_rs::prelude::{Decoder, DecoderBuffer};
use std::{env, path::PathBuf};

use crate::{
    adapters::querier::helpers::{build_search_query, register_with_clip_id},
    core::{
        domain::model::{ClipSearchParams, DataError, PointCloud},
        ports::outbound::data_store::DataStore,
    },
};
#[async_trait::async_trait]
impl DataStore for SessionContext {
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
                .expect("get column by name")
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("downcast as StringArray");
            result.push(clips_id.value(0).to_string())
        }
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

        register_with_clip_id(
            self,
            &base.join("lidar.chunk_0000"),
            ".lidar_top_360fov.parquet",
            "lidar",
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

        // TODO: Register all folders as tables
        println!("Registered parquets from {}", base.display());
        Ok(())
    }

    async fn query_point_clouds(
        &self,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, DataError> {
        load_point_clouds(self, clip_id, num_spins)
            .await
            .map_err(|e| DataError::new(e.to_string()))
    }
}

fn convert_record_batches_to_point_clouds(batches: Vec<RecordBatch>) -> Vec<PointCloud> {
    batches
        .into_iter()
        .flat_map(|batch| {
            PointClouds::try_from(batch)
                .expect("convert record batch to point clouds")
                .0
        })
        .collect()
}

async fn load_point_clouds(
    ctx: &SessionContext,
    clip_id: &str,
    num_spins: usize,
) -> anyhow::Result<Vec<PointCloud>> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/nvidia_physical_dataset/lidar.chunk_0000");

    let path = base.join(format!("{}.lidar_top_360fov.parquet", clip_id));

    if !path.exists() {
        anyhow::bail!("Lidar file not found for clip {}", clip_id);
    }

    let df = ctx
        .sql(&format!(
            "SELECT draco_encoded_pointcloud
         FROM lidar
         WHERE clip_id = '{clip_id}'
         AND spin_index <= {num_spins}",
        ))
        .await?;

    let batches = df.collect().await?;
    Ok(convert_record_batches_to_point_clouds(batches))
}

impl From<DataFusionError> for DataError {
    fn from(value: DataFusionError) -> Self {
        DataError::new(value.message().to_string())
    }
}

// Add newtype to get around orphan-rule.
pub struct PointClouds(pub Vec<PointCloud>);

impl TryFrom<RecordBatch> for PointClouds {
    type Error = DataError;

    fn try_from(value: RecordBatch) -> Result<Self, Self::Error> {
        let draco_col = value
            .column_by_name("draco_encoded_pointcloud")
            .ok_or_else(|| DataError::new("draco column not found".into()))?
            .as_any()
            .downcast_ref::<arrow::array::BinaryViewArray>()
            .ok_or_else(|| DataError::new("draco column is not BinaryViewArray".into()))?;

        let mut point_clouds = Vec::with_capacity(draco_col.len());

        for row in 0..draco_col.len() {
            let draco_bytes = draco_col.value(row);
            let mut buffer = DecoderBuffer::from_buffer(draco_bytes);
            let mut decoder = Decoder::new();

            let mut pc = draco_rs::pointcloud::PointCloud::from_buffer(&mut decoder, &mut buffer)
                .map_err(|e| {
                DataError::new(format!("Draco decode failed at row {}: {:?}", row, e))
            })?;

            let attr_id = pc
                .get_named_attribute_id(GeometryAttribute_Type::POSITION, 0)
                .ok_or_else(|| DataError::new("No position attribute".into()))?;

            let num_points = pc.num_points();
            let mut points = Vec::with_capacity(num_points as usize);
            for i in 0..num_points {
                points.push(pc.get_point_alloc::<f32, 3>(attr_id, i));
            }

            point_clouds.push(PointCloud { points });
        }

        Ok(PointClouds(point_clouds))
    }
}

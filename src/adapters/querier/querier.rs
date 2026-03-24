use datafusion::{
    arrow::{
        self,
        array::{Array, StringArray},
    },
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};
use draco_rs::pointcloud::PointCloud;
use draco_rs::prelude::ffi::draco::GeometryAttribute_Type;
use draco_rs::prelude::{Decoder, DecoderBuffer};
use std::{env, path::PathBuf};

use crate::{
    adapters::querier::helpers::{build_search_query, register_with_clip_id},
    core::{
        domain::model::{ClipSearchParams, DataError},
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
                .expect("msg")
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("msg");
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

        println!("Registered parquets from {}", base.display());
        Ok(())
    }

    async fn query_point_cloud(
        &self,
        clip_id: &str,
        spin_index: usize,
    ) -> Result<Vec<[f32; 3]>, DataError> {
        load_point_cloud(self, clip_id, spin_index)
            .await
            .map_err(|e| DataError::new(e.to_string()))
    }
}

async fn load_point_cloud(
    ctx: &SessionContext,
    clip_id: &str,
    spin_index: usize,
) -> anyhow::Result<Vec<[f32; 3]>> {
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
         AND spin_index = {spin_index}",
        ))
        .await?;

    let batches = df.collect().await?;
    let batch = batches
        .first()
        .ok_or_else(|| anyhow::anyhow!("No spin {} for clip {}", spin_index, clip_id))?;
    let draco_col = batch
        .column_by_name("draco_encoded_pointcloud")
        .ok_or_else(|| anyhow::anyhow!("draco column not found"))?
        .as_any()
        .downcast_ref::<arrow::array::BinaryViewArray>()
        .ok_or_else(|| anyhow::anyhow!("draco column is not BinaryViewArray"))?;

    let draco_bytes = draco_col.value(0);

    let mut buffer = DecoderBuffer::from_buffer(draco_bytes);
    let mut decoder = Decoder::new();
    let mut pc = PointCloud::from_buffer(&mut decoder, &mut buffer)
        .map_err(|e| anyhow::anyhow!("Draco decode failed: {:?}", e))?;

    let attr_id = pc.get_named_attribute_id(GeometryAttribute_Type::POSITION, 0);

    let num_points = pc.num_points();
    println!("number of points, {:#?}", num_points);
    let mut points: Vec<[f32; 3]> = Vec::with_capacity(num_points as usize);

    for i in 0..num_points {
        let p = pc.get_point_alloc::<f32, 3>(attr_id.expect("attri id"), i);
        points.push(p);
    }

    Ok(points)
}

impl From<DataFusionError> for DataError {
    fn from(value: DataFusionError) -> Self {
        DataError::new(value.message().to_string())
    }
}

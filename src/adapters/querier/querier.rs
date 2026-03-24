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

fn convert_record_batches_to_point_clouds(batches: &mut Vec<RecordBatch>) -> Vec<PointCloud> {
    let mut point_clouds: Vec<PointCloud> = vec![];

    for batch in batches.iter_mut() {
        let draco_col = batch
            .column_by_name("draco_encoded_pointcloud")
            .expect("draco column not found")
            .as_any()
            .downcast_ref::<arrow::array::BinaryViewArray>()
            .expect("draco column is not BinaryViewArray");

        for i in 0..draco_col.len() {
            let draco_bytes = draco_col.value(i);
            let mut buffer = DecoderBuffer::from_buffer(draco_bytes);
            let mut decoder = Decoder::new();
            match draco_rs::pointcloud::PointCloud::from_buffer(&mut decoder, &mut buffer) {
                Ok(mut pc) => {
                    let attr_id = pc.get_named_attribute_id(GeometryAttribute_Type::POSITION, 0);

                    let num_points = pc.num_points();
                    let mut points: Vec<[f32; 3]> = Vec::with_capacity(num_points as usize);

                    for i in 0..num_points {
                        let p = pc.get_point_alloc::<f32, 3>(attr_id.expect("attri id"), i);
                        points.push(p);
                    }

                    point_clouds.push(points.into())
                }
                Err(e) => eprintln!("[warn] failed to decode spin {}: {:?}", i, e),
            }
        }
    }

    println!("number of point clouds, {:#?}", point_clouds.len());
    point_clouds
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

    let mut batches = df.collect().await?;
    let point_clouds = convert_record_batches_to_point_clouds(&mut batches);
    Ok(point_clouds)
}

impl From<DataFusionError> for DataError {
    fn from(value: DataFusionError) -> Self {
        DataError::new(value.message().to_string())
    }
}

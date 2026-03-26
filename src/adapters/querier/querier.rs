use datafusion::arrow::datatypes::Float64Type;
use datafusion::{
    arrow::{
        self,
        array::{Array, AsArray, RecordBatch, StringArray},
    },
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};
use draco_rs::prelude::ffi::draco::GeometryAttribute_Type;
use draco_rs::prelude::{Decoder, DecoderBuffer};
use log::{info, warn};

use crate::{
    adapters::querier::helpers::{build_search_query, register_with_clip_id},
    core::{
        build_dataset_path,
        domain::model::{ClipSearchParams, DataError, EgoMotion, PointCloud},
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
        let base = build_dataset_path();

        for (folder, file_ext, table_name) in [
            ("egomotion.chunk_0000", ".egomotion.parquet", "ego_motion"),
            (
                "camera_front_wide_120fov.chunk_0000",
                ".camera_front_wide_120fov.timestamps.parquet",
                "camera_timestamps",
            ),
            ("lidar.chunk_0000", ".lidar_top_360fov.parquet", "lidar"),
        ] {
            register_with_clip_id(self, &base.join(folder), file_ext, table_name).await?;
        }

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

        // TODO: Register all folders as tables
        info!("Registered parquets from {}", base.display());
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

    async fn query_ego_motion(&self, clip_id: &str) -> Result<Vec<EgoMotion>, DataError> {
        load_ego_motion(self, clip_id)
            .await
            .map_err(|e| DataError::new(e.to_string()))
    }
}

fn convert_record_batches_to_transforms(batches: Vec<RecordBatch>) -> Vec<EgoMotion> {
    batches
        .into_iter()
        .flat_map(|batch| {
            let get_f64 = |name: &str| {
                batch
                    .column_by_name(name)
                    .map(|col| col.as_primitive::<Float64Type>())
            };

            let (x, y, z, qx, qy, qz, qw) = match (
                get_f64("x"),
                get_f64("y"),
                get_f64("z"),
                get_f64("qx"),
                get_f64("qy"),
                get_f64("qz"),
                get_f64("qw"),
            ) {
                (Some(x), Some(y), Some(z), Some(qx), Some(qy), Some(qz), Some(qw)) => {
                    (x, y, z, qx, qy, qz, qw)
                }
                _ => {
                    warn!("batch missing expected columns, skipping");
                    return vec![];
                }
            };

            (0..batch.num_rows())
                .map(|i| EgoMotion {
                    position: [x.value(i) as f32, y.value(i) as f32, z.value(i) as f32],
                    rotation: [
                        qx.value(i) as f32,
                        qy.value(i) as f32,
                        qz.value(i) as f32,
                        qw.value(i) as f32,
                    ],
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

async fn load_ego_motion(ctx: &SessionContext, clip_id: &str) -> anyhow::Result<Vec<EgoMotion>> {
    let path = build_dataset_path()
        .join("egomotion.chunk_0000")
        .join(format!("{}.egomotion.parquet", clip_id));

    if !path.exists() {
        anyhow::bail!("ego_motion file not found for clip {}", clip_id);
    }
    let df = ctx
        .sql(&format!(
            "SELECT *
         FROM ego_motion
         WHERE clip_id = '{clip_id}'"
        ))
        .await?;

    let batches = df.collect().await?;
    Ok(convert_record_batches_to_transforms(batches))
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
    let path = build_dataset_path()
        .join("lidar.chunk_0000")
        .join(format!("{}.lidar_top_360fov.parquet", clip_id));

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

// Use newtype to get around orphan-rule.
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

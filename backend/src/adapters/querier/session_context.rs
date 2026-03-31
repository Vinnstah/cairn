use datafusion::{
    arrow::{
        self,
        array::{Array, AsArray, RecordBatch, StringArray},
    },
    prelude::{ParquetReadOptions, SessionContext},
};
use draco_rs::prelude::ffi::draco::GeometryAttribute_Type;
use draco_rs::prelude::{Decoder, DecoderBuffer};
use log::info;
use shared::error::CairnError;
use shared::{ClipSearchParams, ColumnInfo, SchemaResponse};

use crate::{
    adapters::querier::helpers::{build_search_query, register_with_clip_id},
    core::{
        build_dataset_path,
        domain::model::{BoundingBox, EgoMotion, PointCloud},
        ports::outbound::data_store::DataStore,
    },
};
use crate::{
    adapters::querier::loaders::{load_ego_motion, load_point_clouds},
    error::ServerError,
};

#[async_trait::async_trait]
impl DataStore for SessionContext {
    async fn query_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> Result<Vec<String>, ServerError> {
        info!("query clips with params");
        let df = self
            .sql(build_search_query(params.clone()).as_str())
            .await?;
        let batches = df.collect().await?;
        if batches.len() == 0 {
            return Err(ServerError(CairnError::Generic {
                reason: format!("No clips found with params, {:#?}", params),
            }));
        }
        let mut result = vec![];
        for batch in batches {
            let clips_id = batch
                .column_by_name("clip_id")
                .ok_or(CairnError::MissingColumn("clip_id"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(CairnError::FailedToConvertToType("StringArray".to_owned()))?;
            result.push(clips_id.value(0).to_string())
        }
        Ok(result)
    }

    async fn register_tables(&self) -> Result<(), ServerError> {
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
            (
                "obstacle.offline.chunk_0000",
                ".obstacle.offline.parquet",
                "obstacles",
            ),
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
    ) -> Result<Vec<PointCloud>, ServerError> {
        info!("query point clouds");
        load_point_clouds(self, clip_id, num_spins).await
    }

    async fn query_ego_motion(&self, clip_id: &str) -> Result<Vec<EgoMotion>, ServerError> {
        info!("query ego motion");
        load_ego_motion(self, clip_id).await
    }

    async fn query_schema(&self) -> Result<SchemaResponse, ServerError> {
        let df = self
            .sql("SELECT * FROM ego_motion LIMIT 0")
            .await
            .map_err(|err| CairnError::QueryFailed(err.to_string()))?;
        let schema = df
            .schema()
            .fields()
            .iter()
            .map(|f| ColumnInfo {
                name: f.name().clone(),
                data_type: f.data_type().to_string(),
            })
            .collect();

        let df = self
        .sql("SELECT DISTINCT label_class FROM obstacles WHERE label_class IS NOT NULL ORDER BY label_class")
        .await?;

        let batches = df.collect().await?;

        let mut classes = vec![];
        for batch in &batches {
            let col = batch
                .column_by_name("label_class")
                .ok_or(CairnError::MissingColumn("label_class"))?
                .as_any()
                .downcast_ref::<arrow::array::StringViewArray>()
                .ok_or(CairnError::FailedToConvertToType("StringViewArray".into()))?;

            for i in 0..col.len() {
                if !col.is_null(i) {
                    classes.push(col.value(i).to_string());
                }
            }
        }
        Ok(SchemaResponse::new(schema, classes))
    }

    async fn query_bounding_boxes(&self, clip_id: &str) -> Result<Vec<BoundingBox>, ServerError> {
        info!("query bounding boxes for clip {}", clip_id);

        let df = self
            .sql(&format!(
                "SELECT
            track_id,
            label_class,
            CAST(timestamp_us AS BIGINT) AS timestamp_us,
            center_x, center_y, center_z,
            size_x,   size_y,   size_z,
            orientation_x, orientation_y,
            orientation_z, orientation_w
         FROM obstacles
         WHERE clip_id = '{clip_id}'
           AND label_class IS NOT NULL
         ORDER BY timestamp_us"
            ))
            .await?;

        info!("bounding boxes sql executed for clip {}", clip_id);
        let batches = df.collect().await?;
        info!("collected {} bounding box batches", batches.len());
        let mut boxes = vec![];
        if batches.len() > 0 {
            info!("found bounding boxes");
        }
        for batch in &batches {
            let get_f64 = |name: &str| {
                batch
                    .column_by_name(name)
                    .map(|c| c.as_primitive::<datafusion::arrow::datatypes::Float64Type>())
            };
            let get_str = |name: &str| {
                batch
                    .column_by_name(name)
                    .and_then(|c| c.as_any().downcast_ref::<arrow::array::StringViewArray>())
            };
            let get_i64 = |name: &str| {
                batch
                    .column_by_name(name)
                    .map(|c| c.as_primitive::<datafusion::arrow::datatypes::Int64Type>())
            };

            let (cx, cy, cz) = match (
                get_f64("center_x"),
                get_f64("center_y"),
                get_f64("center_z"),
            ) {
                (Some(x), Some(y), Some(z)) => (x, y, z),
                _ => continue,
            };
            let (sx, sy, sz) = match (get_f64("size_x"), get_f64("size_y"), get_f64("size_z")) {
                (Some(x), Some(y), Some(z)) => (x, y, z),
                _ => continue,
            };
            let (ox, oy, oz, ow) = match (
                get_f64("orientation_x"),
                get_f64("orientation_y"),
                get_f64("orientation_z"),
                get_f64("orientation_w"),
            ) {
                (Some(x), Some(y), Some(z), Some(w)) => (x, y, z, w),
                _ => continue,
            };

            let track_ids = get_str("track_id");
            let label_classes = get_str("label_class");
            let timestamps = get_i64("timestamp_us");

            for i in 0..batch.num_rows() {
                boxes.push(BoundingBox {
                    track_id: track_ids
                        .map(|a| a.value(i).to_string())
                        .unwrap_or_default(),
                    label_class: label_classes
                        .map(|a| a.value(i).to_string())
                        .unwrap_or_default(),
                    timestamp_us: timestamps.map(|a| a.value(i)).unwrap_or(0),
                    center: [cx.value(i) as f32, cy.value(i) as f32, cz.value(i) as f32],
                    size: [sx.value(i) as f32, sy.value(i) as f32, sz.value(i) as f32],
                    rotation: [
                        ox.value(i) as f32,
                        oy.value(i) as f32,
                        oz.value(i) as f32,
                        ow.value(i) as f32,
                    ],
                });
            }
        }
        info!(
            "bounding boxes span {} distinct timestamps",
            boxes
                .iter()
                .map(|b| b.timestamp_us)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
        if let Some(first) = boxes.first() {
            info!("first bbox timestamp_us: {}", first.timestamp_us);
        }
        Ok(boxes)
    }
}

// Use newtype to get around orphan-rule.
pub struct PointClouds(pub Vec<PointCloud>);

impl TryFrom<RecordBatch> for PointClouds {
    type Error = CairnError;

    fn try_from(value: RecordBatch) -> Result<Self, Self::Error> {
        let draco_col = value
            .column_by_name("draco_encoded_pointcloud")
            .ok_or(CairnError::MissingColumn("draco_encoded_pointcloud"))?
            .as_any()
            .downcast_ref::<arrow::array::BinaryViewArray>()
            .ok_or(CairnError::FailedToConvertToType(
                "BinaryViewArray".to_owned(),
            ))?;

        let mut point_clouds = Vec::with_capacity(draco_col.len());

        for row in 0..draco_col.len() {
            let draco_bytes = draco_col.value(row);
            let mut buffer = DecoderBuffer::from_buffer(draco_bytes);
            let mut decoder = Decoder::new();

            let mut pc = draco_rs::pointcloud::PointCloud::from_buffer(&mut decoder, &mut buffer)
                .map_err(|e| CairnError::Generic {
                reason: format!("Draco decode failed at row {}: {:?}", row, e),
            })?;

            let attr_id = pc
                .get_named_attribute_id(GeometryAttribute_Type::POSITION, 0)
                .ok_or_else(|| CairnError::Generic {
                    reason: "No position attribute".into(),
                })?;

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

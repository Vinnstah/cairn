use std::sync::Arc;

use datafusion::{
    arrow::{
        self,
        array::{Array, AsArray, RecordBatch, StringArray},
        datatypes::Schema,
    },
    prelude::{ParquetReadOptions, SessionContext},
};
use draco_rs::prelude::ffi::draco::GeometryAttribute_Type;
use draco_rs::prelude::{Decoder, DecoderBuffer};
use log::{info, warn};
use shared::{ClipSearchParams, ColumnInfo, SchemaResponse};
use shared::{TableSchema, error::CairnError};

use crate::{
    adapters::querier::helpers::{build_search_query, register_with_clip_id},
    core::{
        domain::{
            config::{Config, Dataset, FieldDefinition, SchemaDefinition},
            model::{BoundingBox, EgoMotion, PointCloud},
        },
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
        if batches.is_empty() {
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

    async fn register_tables(&self, datasets: Vec<Dataset>) -> Result<(), ServerError> {
        info!("Registering datasets");
        for dataset in datasets {
            match dataset.characteristics.semantics.clip_id.is_some() {
                true => {
                    let _ = self
                        .register_parquet(
                            &dataset.name,
                            dataset.path.to_str().expect("convert path to str"),
                            ParquetReadOptions::default(),
                        )
                        .await;
                }
                false => {
                    register_with_clip_id(
                        self,
                        dataset.path.as_path(),
                        &dataset.file_ext,
                        &dataset.name,
                    )
                    .await?;
                }
            }

            info!("Registered dataset from {}", dataset.name);
        }
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

    async fn query_schema(&self, config: &Config) -> Result<SchemaResponse, ServerError> {
        info!["querying schemas"];
        let mut tables = vec![];

        for dataset in &config.datasets {
            let df = match self
                .sql(&format!("SELECT * FROM {} LIMIT 0", dataset.name))
                .await
            {
                Ok(df) => df,
                Err(e) => {
                    warn!("could not query schema for table {}: {}", dataset.name, e);
                    continue;
                }
            };

            let schema = df.schema();
            let columns = schema
                .fields()
                .iter()
                .map(|f| ColumnInfo {
                    name: f.name().clone(),
                    data_type: f.data_type().to_string(),
                    nullable: f.is_nullable(),
                })
                .collect();

            tables.push(TableSchema {
                table_name: dataset.name.clone(),
                columns,
            });
        }
        info!["found {} schemas", tables.len()];
        let label_classes = self.query_label_classes(config).await?;

        Ok(SchemaResponse {
            tables,
            label_classes,
        })
    }

    async fn query_label_classes(&self, config: &Config) -> Result<Vec<String>, ServerError> {
        let dataset = config
            .datasets
            .iter()
            .find(|dataset| dataset.characteristics.contains_classes.is_some())
            .expect("find label dataset");

        let label_class_semantic = dataset
            .characteristics
            .semantics
            .label_class
            .as_ref()
            .expect("get label class semantic");

        let table = &dataset.name;

        let df = self
            .sql(&format!(
                "SELECT DISTINCT {:?} FROM {} WHERE {:?} IS NOT NULL ORDER BY {:?}",
                &label_class_semantic, table, &label_class_semantic, &label_class_semantic
            ))
            .await?;

        let batches = df.collect().await?;
        let mut classes = vec![];

        for batch in &batches {
            let col = batch
                .column_by_name(&label_class_semantic)
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

        Ok(classes)
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
        if !batches.is_empty() {
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

    async fn load_schema(&self, dataset: &Dataset) -> SchemaDefinition {
        let schema = self
            .table(dataset.name.as_str())
            .await
            .unwrap()
            .schema()
            .clone();
        schema.inner().into()
    }
}

// Use newtype to get around orphan-rule.
#[derive(Debug)]
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
        let ts_col = value
            .column_by_name("spin_start_timestamp")
            .ok_or(CairnError::MissingColumn("spin_start_timestamp"))?
            .as_primitive::<datafusion::arrow::datatypes::Int64Type>();

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

            point_clouds.push(PointCloud {
                points,
                spin_start_timestamp: ts_col.value(row),
            });
        }

        Ok(PointClouds(point_clouds))
    }
}

impl From<&Arc<Schema>> for SchemaDefinition {
    fn from(value: &Arc<Schema>) -> Self {
        SchemaDefinition {
            fields: value
                .fields()
                .iter()
                .map(|field| FieldDefinition {
                    name: field.name().to_owned(),
                    data_type: field.data_type().to_string(),
                    nullable: field.is_nullable(),
                    metadata: field.metadata().to_owned(),
                })
                .collect(),
            metadata: value.metadata().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use datafusion::arrow::datatypes::{DataType, Field, Schema};

    use super::*;

    #[test]
    fn try_from_missing_column_returns_error() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "clip_id",
            DataType::Utf8,
            false,
        )]));
        let batch =
            RecordBatch::try_new(schema, vec![Arc::new(StringArray::from(vec!["abc"]))]).unwrap();

        let result = PointClouds::try_from(batch);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing column in result: 'draco_encoded_pointcloud'".to_owned()
        );
    }
}

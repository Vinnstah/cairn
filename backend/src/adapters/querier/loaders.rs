use datafusion::{
    arrow::{
        array::{AsArray, RecordBatch},
        datatypes::Float64Type,
    },
    prelude::{ParquetReadOptions, SessionContext},
};
use log::{info, warn};
use rayon::prelude::*;
use shared::error::CairnError;

use crate::{
    adapters::querier::session_context::PointClouds,
    core::{
        build_dataset_path,
        domain::model::{EgoMotion, PointCloud},
    },
    error::ServerError,
};

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

pub async fn load_ego_motion(
    ctx: &SessionContext,
    clip_id: &str,
) -> Result<Vec<EgoMotion>, ServerError> {
    let path = build_dataset_path()
        .join("egomotion.chunk_0000")
        .join(format!("{}.egomotion.parquet", clip_id));

    if !path.exists() {
        return Err(CairnError::ClipNotFound(clip_id.to_owned()).into());
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
        .into_par_iter() // rayon parallel iterator
        .flat_map(|batch| {
            PointClouds::try_from(batch)
                .expect("convert record batch to point clouds")
                .0
        })
        .collect()
}

pub async fn load_point_clouds(
    ctx: &SessionContext,
    clip_id: &str,
    num_spins: usize,
) -> Result<Vec<PointCloud>, ServerError> {
    info!("load_point_clouds start for clip {}", clip_id);

    let path = build_dataset_path()
        .join("lidar.chunk_0000")
        .join(format!("{}.lidar_top_360fov.parquet", clip_id));

    if !path.exists() {
        warn!("lidar file not found for clip {}, skipping", clip_id);
        return Ok(vec![]);
    }

    // Register just this clip's file as a temp table
    let table_name = format!("lidar_clip_{}", &clip_id[..8]);
    ctx.register_parquet(
        &table_name,
        path.to_str().unwrap(),
        ParquetReadOptions::default(),
    )
    .await?;

    let df = ctx
        .sql(&format!(
            "SELECT spin_start_timestamp, draco_encoded_pointcloud
        FROM {table_name}
        WHERE spin_index <= {num_spins}",
        ))
        .await?;

    info!("sql executed for clip {}", clip_id);
    let batches = df.collect().await?;
    info!("collected {} batches for clip {}", batches.len(), clip_id);

    // Deregister to avoid collision on next call
    ctx.deregister_table(&table_name)?;

    if batches.is_empty() {
        warn!("no lidar data found for clip {}", clip_id);
        return Ok(vec![]);
    }

    let point_clouds = convert_record_batches_to_point_clouds(batches);
    info!(
        "decoded {} point clouds for clip {}",
        point_clouds.len(),
        clip_id
    );

    Ok(point_clouds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::Float64Array;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    fn make_ego_batch(x: f64, y: f64, z: f64) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("x", DataType::Float64, true),
            Field::new("y", DataType::Float64, true),
            Field::new("z", DataType::Float64, true),
            Field::new("qx", DataType::Float64, true),
            Field::new("qy", DataType::Float64, true),
            Field::new("qz", DataType::Float64, true),
            Field::new("qw", DataType::Float64, true),
        ]));

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Float64Array::from(vec![x])),
                Arc::new(Float64Array::from(vec![y])),
                Arc::new(Float64Array::from(vec![z])),
                Arc::new(Float64Array::from(vec![0.0])),
                Arc::new(Float64Array::from(vec![0.0])),
                Arc::new(Float64Array::from(vec![0.0])),
                Arc::new(Float64Array::from(vec![1.0])),
            ],
        )
        .unwrap()
    }

    #[test]
    fn converts_batch_to_ego_motion() {
        let batch = make_ego_batch(1.0, 2.0, 3.0);
        let result = convert_record_batches_to_transforms(vec![batch]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn empty_batch_returns_empty_vec() {
        let result = convert_record_batches_to_transforms(vec![]);
        assert!(result.is_empty());
    }
}

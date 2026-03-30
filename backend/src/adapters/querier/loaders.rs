use datafusion::{
    arrow::{
        array::{AsArray, RecordBatch},
        datatypes::Float64Type,
    },
    prelude::SessionContext,
};
use log::warn;

use crate::{
    adapters::querier::session_context::PointClouds,
    core::{
        build_dataset_path,
        domain::model::{EgoMotion, PointCloud},
    },
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
) -> anyhow::Result<Vec<EgoMotion>> {
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

pub async fn load_point_clouds(
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

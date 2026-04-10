use log::info;
use rerun::{
    AsComponents, Points3D, Position3D, RecordingStream, SerializedComponentBatch,
    external::re_log::ResultExt,
};
use shared::error::CairnError;

use crate::{
    core::{
        domain::model::{BoundingBox, EgoMotion, PointCloud},
        ports::outbound::scene_logger::SceneLogger,
    },
    error::ServerError,
};

#[async_trait::async_trait]
impl SceneLogger for RecordingStream {
    async fn replay_video(&self, video: rerun::archetypes::AssetVideo) -> Result<(), ServerError> {
        info!("replaying video, {:#?}", video);
        self.log_static("video", &video).ok_or_log_error();

        // Send automatically determined video frame timestamps.
        let frame_timestamps_nanos =
            video
                .read_frame_timestamps_nanos()
                .map_err(|err| CairnError::Generic {
                    reason: err.to_string(),
                })?;
        let video_timestamps_nanos = frame_timestamps_nanos
            .iter()
            .copied()
            .map(rerun::components::VideoTimestamp::from_nanos)
            .collect::<Vec<_>>();
        let time_column = rerun::TimeColumn::new_duration_nanos(
            "video_time",
            // Note timeline values don't have to be the same as the video timestamps.
            frame_timestamps_nanos,
        );

        self.send_columns(
            "video",
            [time_column],
            rerun::VideoFrameReference::update_fields()
                .with_many_timestamp(video_timestamps_nanos)
                .columns_of_unit_batches()
                .map_err(|err| CairnError::Generic {
                    reason: err.to_string(),
                })?,
        )?;

        Ok(())
    }

    async fn replay_point_clouds(
        &self,
        point_clouds: Vec<PointCloud>,
        time_offset_us: i64,
    ) -> Result<(), ServerError> {
        for pc in &point_clouds {
            self.set_timestamp_secs_since_epoch(
                "ego_time",
                (pc.spin_start_timestamp + time_offset_us) as f64 / 1_000_000.0,
            );

            // Downsample: take every Nth point
            // Full cloud is ~254k points — 1 in 10 gives ~25k which is plenty for viz
            let downsampled: Vec<[f32; 3]> = pc.points.iter().step_by(10).copied().collect();

            self.log(
                "world/lidar",
                &rerun::archetypes::Points3D::new(downsampled),
            )
            .map_err(|e| CairnError::Generic {
                reason: e.to_string(),
            })?;
        }
        Ok(())
    }

    async fn replay_ego_motion(
        &self,
        ego_motion: Vec<EgoMotion>,
        time_offset_us: i64,
    ) -> Result<(), ServerError> {
        for (i, sample) in ego_motion.iter().enumerate() {
            self.set_time_sequence("ego_step", i as i64);
            self.log(
                "world/vehicle",
                &rerun::archetypes::Transform3D::from_translation_rotation(
                    sample.position,
                    rerun::Quaternion::from_xyzw(sample.rotation),
                ),
            )?;
        }
        let positions: Vec<[f32; 3]> = ego_motion.iter().map(|e| e.position).collect();

        self.log(
            "world/trajectory",
            &rerun::archetypes::LineStrips3D::new([positions]),
        )?;
        Ok(())
    }

    async fn replay_bounding_boxes(
        &self,
        boxes: Vec<BoundingBox>,
        time_offset_us: i64,
    ) -> Result<(), ServerError> {
        use rerun::TimeColumn;

        // Group by track
        let mut by_track: std::collections::HashMap<String, Vec<&BoundingBox>> =
            std::collections::HashMap::new();
        for b in &boxes {
            let short_id: String = b.track_id.chars().take(8).collect();
            by_track
                .entry(format!("{}/{}", b.label_class, short_id))
                .or_default()
                .push(b);
        }

        for (track_key, mut track_boxes) in by_track {
            track_boxes.sort_by_key(|b| b.timestamp_us);
            let entity_path = format!("world/obstacles/{}", track_key);

            let mut times: Vec<f64> = track_boxes
                .iter()
                .map(|b| (b.timestamp_us + time_offset_us) as f64 / 1_000_000.0)
                .collect();

            let centers: Vec<[f32; 3]> = track_boxes.iter().map(|b| b.center).collect();
            let sizes: Vec<[f32; 3]> = track_boxes.iter().map(|b| b.size).collect();
            let rotations: Vec<rerun::Quaternion> = track_boxes
                .iter()
                .map(|b| rerun::Quaternion::from_xyzw(b.rotation))
                .collect();

            // Log the detections
            let time_column = TimeColumn::new_timestamp_secs_since_epoch("ego_time", times.clone());
            self.send_columns(
                entity_path.clone(),
                [time_column],
                rerun::archetypes::Boxes3D::from_centers_and_sizes(centers, sizes)
                    .with_quaternions(rotations)
                    .columns_of_unit_batches()
                    .unwrap(),
            )?;

            // Log a clear just after the last detection so the box disappears
            // when the track is no longer observed
            let last_ts = track_boxes.last().unwrap().timestamp_us;
            let clear_ts = (last_ts + time_offset_us + 1) as f64 / 1_000_000.0;
            self.set_timestamp_secs_since_epoch("ego_time", clear_ts);
            self.log(entity_path, &rerun::archetypes::Clear::flat())?;
        }

        Ok(())
    }
}

impl From<PointCloud> for Points3D {
    fn from(value: PointCloud) -> Self {
        Points3D::new(value.points)
    }
}

// Needed so that we can log PointCloud directly to Rerun
impl AsComponents for PointCloud {
    fn as_serialized_batches(&self) -> Vec<SerializedComponentBatch> {
        let positions: Vec<Position3D> = self
            .points
            .iter()
            .map(|p| Position3D::new(p[0], p[1], p[2]))
            .collect();

        rerun::archetypes::Points3D::new(positions).as_serialized_batches()
    }
}

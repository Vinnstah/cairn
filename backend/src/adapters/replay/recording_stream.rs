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

    async fn replay_point_clouds(&self, point_clouds: Vec<PointCloud>) -> Result<(), ServerError> {
        for (_, pc) in point_clouds.iter().enumerate() {
            self.set_timestamp_secs_since_epoch(
                "ego_time",
                pc.spin_start_timestamp as f64 / 1_000_000.0,
            );
            self.log("world/lidar", pc)?;
        }
        Ok(())
    }

    async fn replay_ego_motion(&self, ego_motion: Vec<EgoMotion>) -> Result<(), ServerError> {
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

    async fn replay_bounding_boxes(&self, boxes: Vec<BoundingBox>) -> Result<(), ServerError> {
        // Group by track_id
        let mut by_track: std::collections::HashMap<String, Vec<&BoundingBox>> =
            std::collections::HashMap::new();

        for b in &boxes {
            let short_id: String = b.track_id.chars().take(8).collect();
            let key = format!("{}/{}", b.label_class, short_id);
            by_track.entry(key).or_default().push(b);
        }

        for (track_key, track_boxes) in &by_track {
            let entity_path = format!("world/obstacles/{}", track_key);

            // Sort by time
            let mut sorted = track_boxes.to_vec();
            sorted.sort_by_key(|b| b.timestamp_us);

            for (i, b) in sorted.iter().enumerate() {
                // Each box is visible until the next box for this track arrives
                // For the last box, keep it visible for one extra interval
                let next_ts = sorted
                    .get(i + 1)
                    .map(|next| next.timestamp_us)
                    .unwrap_or(b.timestamp_us + 100_000); // 100ms falloff

                // Log box at its timestamp
                self.set_timestamp_secs_since_epoch(
                    "ego_time",
                    b.timestamp_us as f64 / 1_000_000.0,
                );
                self.log(
                    entity_path.as_str(),
                    &rerun::archetypes::Boxes3D::from_centers_and_sizes([b.center], [b.size])
                        .with_quaternions([rerun::Quaternion::from_xyzw(b.rotation)])
                        .with_labels([format!(
                            "{} ({})",
                            b.label_class,
                            track_key.split('/').last().unwrap_or("")
                        )]),
                )?;

                // Log a clear just before the next box arrives so it doesn't
                // persist beyond its valid window
                self.set_timestamp_secs_since_epoch("ego_time", (next_ts - 1) as f64 / 1_000_000.0);
                self.log(entity_path.as_str(), &rerun::archetypes::Clear::flat())?;
            }
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

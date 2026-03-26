use log::info;
use rerun::{
    AsComponents, Points3D, Position3D, RecordingStream, SerializedComponentBatch,
    external::re_log::ResultExt,
};

use crate::core::{domain::model::PointCloud, ports::outbound::scene_logger::SceneLogger};

#[async_trait::async_trait]
impl SceneLogger for RecordingStream {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()> {
        info!("replaying video, {:#?}", video);
        self.log_static("video", &video).ok_or_log_error();

        // Send automatically determined video frame timestamps.
        let frame_timestamps_nanos = video.read_frame_timestamps_nanos()?;
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
                .columns_of_unit_batches()?,
        )?;

        Ok(())
    }

    async fn replay_point_clouds(&self, point_clouds: Vec<PointCloud>) -> anyhow::Result<()> {
        for (spin_index, pc) in point_clouds.iter().enumerate() {
            self.set_time_sequence("spin", spin_index as i64);
            self.log("world/lidar", pc)?;
        }
        Ok(())
    }

    async fn replay_ego_motion(&self, ego_motion: Vec<String>) -> anyhow::Result<()> {
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

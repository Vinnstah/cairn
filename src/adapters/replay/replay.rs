use rerun::RecordingStream;

use crate::core::ports::outbound::replay::Replay;

#[async_trait::async_trait]
impl Replay for RecordingStream {
    async fn visualize_video(&self, video: rerun::archetypes::AssetVideo) -> anyhow::Result<()> {
        println!("replaying video");
        // TODO handle err gracefully
        self.log_static("video", &video);
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
}

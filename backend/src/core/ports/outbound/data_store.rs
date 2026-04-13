use shared::{ClipSearchParams, SchemaResponse};

use crate::{
    core::domain::{
        config::{Config, Dataset, SchemaDefinition},
        model::{BoundingBox, EgoMotion, PointCloud},
    },
    error::ServerError,
};

/// Responsible of connecting to a data-store and querying the underlying data
#[async_trait::async_trait]
pub trait DataStore {
    async fn register_tables(&self, datasets: Vec<Dataset>) -> Result<(), ServerError>;
    async fn query_clips_with_params(
        &self,
        params: ClipSearchParams,
    ) -> Result<Vec<String>, ServerError>;
    async fn query_point_clouds(
        &self,
        config: &Config,
        clip_id: &str,
        num_spins: usize,
    ) -> Result<Vec<PointCloud>, ServerError>;
    async fn query_ego_motion(&self, clip_id: &str) -> Result<Vec<EgoMotion>, ServerError>;
    async fn query_schema(&self, config: &Config) -> Result<SchemaResponse, ServerError>;
    async fn query_bounding_boxes(&self, clip_id: &str) -> Result<Vec<BoundingBox>, ServerError>;
    async fn query_label_classes(&self, config: &Config) -> Result<Vec<String>, ServerError>;
    async fn load_schema(&self, dataset: &Dataset) -> SchemaDefinition;
}

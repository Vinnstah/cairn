use shared::ClipSearchParams;

use crate::error::ServerError;

#[async_trait::async_trait]
pub trait Replay: Send + Sync {
    async fn replay_clips(&self, params: ClipSearchParams) -> Result<(), ServerError>;
}

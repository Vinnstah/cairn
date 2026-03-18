use std::sync::Arc;

use crate::querier::querier::Querier;

pub struct AppState<QR: Querier> {
    pub querier: Arc<QR>,
}

impl<QR: Querier> AppState<QR> {
    pub fn new(querier: Arc<QR>) -> Self {
        Self { querier }
    }
}

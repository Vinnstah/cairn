use std::sync::Arc;

use crate::domain::port::Querier;

pub struct Service<QR: Querier> {
    pub querier: Arc<QR>,
}

impl<QR: Querier> Service<QR> {
    pub fn new(querier: Arc<QR>) -> Self {
        Self { querier }
    }
}

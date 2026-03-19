use std::sync::Arc;

use crate::domain::port::{Querier, RouteDelegator};

pub struct Service<Q: Querier, R: RouteDelegator> {
    pub querier: Arc<Q>,
    pub router: Arc<R>,
}

impl<Q: Querier, R: RouteDelegator> Service<Q, R> {
    pub fn new(querier: Arc<Q>, router: Arc<R>) -> Self {
        Self { querier, router }
    }
}

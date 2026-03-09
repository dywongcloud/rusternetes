use rusternetes_storage::Storage;
use std::sync::Arc;

/// Shared state for the API server
pub struct ApiServerState {
    pub storage: Arc<dyn Storage>,
}

impl ApiServerState {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }
}

use rusternetes_common::auth::TokenManager;
use rusternetes_common::authz::Authorizer;
use rusternetes_common::observability::MetricsRegistry;
use rusternetes_storage::etcd::EtcdStorage;
use std::sync::Arc;

/// Shared state for the API server
pub struct ApiServerState {
    pub storage: Arc<EtcdStorage>,
    pub token_manager: Arc<TokenManager>,
    pub authorizer: Arc<dyn Authorizer>,
    pub metrics: Arc<MetricsRegistry>,
}

impl ApiServerState {
    pub fn new(
        storage: Arc<EtcdStorage>,
        token_manager: Arc<TokenManager>,
        authorizer: Arc<dyn Authorizer>,
        metrics: Arc<MetricsRegistry>,
    ) -> Self {
        Self {
            storage,
            token_manager,
            authorizer,
            metrics,
        }
    }
}

use rusternetes_common::auth::TokenManager;
use rusternetes_common::authz::Authorizer;
use rusternetes_common::observability::MetricsRegistry;
use rusternetes_storage::etcd::EtcdStorage;
use std::sync::Arc;
use crate::ip_allocator::ClusterIPAllocator;

/// Shared state for the API server
pub struct ApiServerState {
    pub storage: Arc<EtcdStorage>,
    pub token_manager: Arc<TokenManager>,
    pub authorizer: Arc<dyn Authorizer>,
    pub metrics: Arc<MetricsRegistry>,
    pub skip_auth: bool,
    pub ip_allocator: Arc<ClusterIPAllocator>,
}

impl ApiServerState {
    pub fn new(
        storage: Arc<EtcdStorage>,
        token_manager: Arc<TokenManager>,
        authorizer: Arc<dyn Authorizer>,
        metrics: Arc<MetricsRegistry>,
        skip_auth: bool,
    ) -> Self {
        Self {
            storage,
            token_manager,
            authorizer,
            metrics,
            skip_auth,
            ip_allocator: Arc::new(ClusterIPAllocator::new()),
        }
    }
}

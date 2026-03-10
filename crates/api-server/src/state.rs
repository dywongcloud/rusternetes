use rusternetes_common::auth::TokenManager;
use rusternetes_common::authz::Authorizer;
use rusternetes_common::observability::MetricsRegistry;
use rusternetes_storage::etcd::EtcdStorage;
use std::sync::Arc;
use crate::ip_allocator::ClusterIPAllocator;
use crate::admission_webhook::AdmissionWebhookManager;
use crate::prometheus_client::PrometheusClient;

/// Shared state for the API server
pub struct ApiServerState {
    pub storage: Arc<EtcdStorage>,
    pub token_manager: Arc<TokenManager>,
    pub authorizer: Arc<dyn Authorizer>,
    pub metrics: Arc<MetricsRegistry>,
    pub skip_auth: bool,
    pub ip_allocator: Arc<ClusterIPAllocator>,
    pub webhook_manager: Arc<AdmissionWebhookManager<EtcdStorage>>,
    pub ca_cert_pem: Option<String>,
    pub prometheus_client: Option<Arc<PrometheusClient>>,
}

impl ApiServerState {
    pub fn new(
        storage: Arc<EtcdStorage>,
        token_manager: Arc<TokenManager>,
        authorizer: Arc<dyn Authorizer>,
        metrics: Arc<MetricsRegistry>,
        skip_auth: bool,
    ) -> Self {
        let webhook_manager = Arc::new(AdmissionWebhookManager::new(storage.clone()));

        Self {
            storage,
            token_manager,
            authorizer,
            metrics,
            skip_auth,
            ip_allocator: Arc::new(ClusterIPAllocator::new()),
            webhook_manager,
            ca_cert_pem: None,
            prometheus_client: None,
        }
    }

    /// Set the CA certificate PEM for distribution to service accounts
    pub fn with_ca_cert(mut self, ca_cert_pem: Option<String>) -> Self {
        self.ca_cert_pem = ca_cert_pem;
        self
    }

    /// Set the Prometheus client for custom metrics
    pub fn with_prometheus_client(mut self, prometheus_client: Option<Arc<PrometheusClient>>) -> Self {
        self.prometheus_client = prometheus_client;
        self
    }
}

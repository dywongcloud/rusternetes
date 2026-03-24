mod admission;
mod admission_webhook;
mod bootstrap;
mod conversion;
mod dynamic_routes;
mod flow_control;
mod handlers;
mod ip_allocator;
mod middleware;
mod openapi;
mod patch;
mod prometheus_client;
mod response;
mod router;
mod spdy;
mod spdy_handlers;
mod state;
mod streaming;
mod watch_cache;

use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use prometheus_client::PrometheusClient;
use rusternetes_common::auth::TokenManager;
use rusternetes_common::authz::RBACAuthorizer;
use rusternetes_common::observability::MetricsRegistry;
use rusternetes_common::tls::TlsConfig;
use rusternetes_storage::etcd::EtcdStorage;
use rusternetes_storage::Storage;
use state::ApiServerState;
use tracing::debug;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(name = "rusternetes-api-server")]
#[command(about = "Rusternetes API Server - Kubernetes API reimplemented in Rust")]
struct Args {
    /// Address to bind to
    #[arg(long, default_value = "0.0.0.0:6443")]
    bind_address: String,

    /// Etcd endpoints (comma-separated)
    #[arg(long, default_value = "http://localhost:2379")]
    etcd_servers: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// JWT secret for service account tokens
    #[arg(long, default_value = "rusternetes-secret-change-in-production")]
    jwt_secret: String,

    /// Enable TLS/HTTPS
    #[arg(long)]
    tls: bool,

    /// TLS certificate file (PEM format)
    #[arg(long)]
    tls_cert_file: Option<String>,

    /// TLS private key file (PEM format)
    #[arg(long)]
    tls_key_file: Option<String>,

    /// Generate self-signed certificate if TLS files not provided
    #[arg(long)]
    tls_self_signed: bool,

    /// Subject Alternative Names for self-signed cert (comma-separated)
    #[arg(long, default_value = "localhost,127.0.0.1")]
    tls_san: String,

    /// Skip authentication and authorization (INSECURE - development only)
    #[arg(long)]
    skip_auth: bool,

    /// Prometheus server URL for custom metrics (optional)
    #[arg(long)]
    prometheus_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt().with_max_level(level).init();

    info!("Starting Rusternetes API Server");

    // Parse etcd endpoints
    let etcd_endpoints: Vec<String> = args
        .etcd_servers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Initialize storage
    info!("Connecting to etcd: {:?}", etcd_endpoints);
    let storage = Arc::new(EtcdStorage::new(etcd_endpoints).await?);

    // Initialize TokenManager
    info!("Initializing TokenManager with JWT secret");
    let token_manager = Arc::new(TokenManager::new(args.jwt_secret.as_bytes()));

    // Initialize Authorizer (RBAC or AlwaysAllow based on skip_auth)
    let authorizer: Arc<dyn rusternetes_common::authz::Authorizer> = if args.skip_auth {
        warn!("⚠️  AUTHENTICATION AND AUTHORIZATION DISABLED - INSECURE MODE");
        warn!("⚠️  Using AlwaysAllowAuthorizer - all requests will be permitted");
        warn!("⚠️  This should ONLY be used in development/testing environments");
        Arc::new(rusternetes_common::authz::AlwaysAllowAuthorizer)
    } else {
        info!("Initializing RBAC Authorizer");
        Arc::new(RBACAuthorizer::new(storage.clone()))
    };

    // Initialize Metrics Registry
    info!("Initializing Metrics Registry");
    let metrics = Arc::new(MetricsRegistry::new().with_api_server_metrics()?);

    // Load or generate TLS config (if TLS is enabled)
    let ca_cert_pem = if args.tls {
        info!("TLS enabled - loading/generating certificates");

        let tls_config = if let (Some(cert_file), Some(key_file)) =
            (args.tls_cert_file.clone(), args.tls_key_file.clone())
        {
            info!(
                "Loading TLS certificate from {} and key from {}",
                cert_file, key_file
            );
            TlsConfig::from_pem_files(&cert_file, &key_file)?
        } else if args.tls_self_signed {
            warn!("Generating self-signed certificate - NOT suitable for production!");
            let sans: Vec<String> = args
                .tls_san
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            info!("Self-signed cert SANs: {:?}", sans);
            TlsConfig::generate_self_signed("rusternetes-api", sans)?
        } else {
            anyhow::bail!("TLS enabled but no certificate provided. Use --tls-cert-file and --tls-key-file, or --tls-self-signed");
        };

        tls_config.cert_pem.clone()
    } else {
        None
    };

    // Bootstrap kubernetes Service Endpoints with dynamic IP discovery
    let api_port = args
        .bind_address
        .split(':')
        .last()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(6443);

    if let Err(e) = bootstrap::bootstrap_kubernetes_service(storage.clone(), api_port).await {
        warn!(
            "Failed to bootstrap kubernetes Service Endpoints: {}. Continuing anyway.",
            e
        );
    }

    // Initialize Prometheus client for custom metrics (if URL provided)
    let prometheus_client = if let Some(url) = args.prometheus_url {
        info!("Initializing Prometheus client: {}", url);
        match PrometheusClient::new(url.clone()) {
            Ok(client) => {
                info!("Prometheus client initialized successfully");
                Some(Arc::new(client))
            }
            Err(e) => {
                warn!("Failed to initialize Prometheus client: {}. Custom metrics will return mock data.", e);
                None
            }
        }
    } else {
        info!("Prometheus URL not provided, custom metrics will return mock data");
        None
    };

    // Create shared state with CA certificate and Prometheus client
    let state = Arc::new(
        ApiServerState::new(storage, token_manager, authorizer, metrics, args.skip_auth)
            .with_ca_cert(ca_cert_pem)
            .with_prometheus_client(prometheus_client),
    );

    // Pre-allocate ClusterIPs from existing services to prevent collisions after restart
    {
        let existing_services: Vec<rusternetes_common::resources::Service> = Storage::list(
            state.storage.as_ref(),
            "/registry/services/",
        )
        .await
        .unwrap_or_default();
        for svc in &existing_services {
            if let Some(ref ip) = svc.spec.cluster_ip {
                if ip != "None" && !ip.is_empty() {
                    state.ip_allocator.mark_allocated(ip.clone());
                    debug!("Pre-allocated ClusterIP {} for existing service {}", ip, svc.metadata.name);
                }
            }
        }
        info!("Pre-allocated {} ClusterIPs from existing services", existing_services.len());
    }

    // Build router
    let app = router::build_router(state);

    // Start server (with or without TLS)
    if args.tls {
        info!("TLS enabled - starting HTTPS server");

        // Reload TLS config for server
        let tls_config =
            if let (Some(cert_file), Some(key_file)) = (args.tls_cert_file, args.tls_key_file) {
                TlsConfig::from_pem_files(&cert_file, &key_file)?
            } else {
                // Must be self-signed
                let sans: Vec<String> = args
                    .tls_san
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                TlsConfig::generate_self_signed("rusternetes-api", sans)?
            };

        let rustls_config = RustlsConfig::from_config(tls_config.into_server_config()?);

        info!("HTTPS server listening on {}", args.bind_address);
        axum_server::bind_rustls(args.bind_address.parse()?, rustls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        info!("TLS disabled - starting HTTP server (not recommended for production)");
        info!("API Server listening on {}", args.bind_address);
        let listener = tokio::net::TcpListener::bind(&args.bind_address).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}

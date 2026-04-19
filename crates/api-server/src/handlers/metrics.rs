use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::Utc;
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{
        ContainerMetrics, NodeMetrics, NodeMetricsMetadata, PodMetrics, PodMetricsMetadata,
    },
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Query Docker for real container CPU/memory usage via container inspect.
async fn collect_node_usage_from_docker(node_name: &str) -> (String, String) {
    use futures::StreamExt;

    let docker = match bollard::Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(_) => return ("0m".to_string(), "0Mi".to_string()),
    };

    let opts = bollard::container::ListContainersOptions::<String> {
        all: false,
        ..Default::default()
    };

    let containers = match docker.list_containers(Some(opts)).await {
        Ok(c) => c,
        Err(_) => return ("0m".to_string(), "0Mi".to_string()),
    };

    // Filter to pod containers (exclude pause and infrastructure)
    let pod_containers: Vec<_> = containers.iter().filter(|c| {
        let image = c.image.as_deref().unwrap_or("");
        !image.contains("pause") && !image.contains("rusternetes-")
    }).collect();

    let mut total_cpu_nano: u64 = 0;
    let mut total_memory_bytes: u64 = 0;
    let mut sampled = 0u64;

    // Sample stats from up to 5 containers (each takes ~1s for 2 samples)
    for container in pod_containers.iter().take(5) {
        let id = match &container.id {
            Some(id) => id.clone(),
            None => continue,
        };

        // Get two stats samples so the second has valid precpu_stats for delta calculation
        let stats_opts = bollard::container::StatsOptions {
            stream: true,
            one_shot: false,
        };
        let mut stats_stream = docker.stats(&id, Some(stats_opts));

        // Skip first sample (precpu_stats are zeros), use second
        let _ = stats_stream.next().await;
        if let Some(Ok(stats)) = stats_stream.next().await {
            // CPU: calculate delta between current and previous sample
            let total_usage = stats.cpu_stats.cpu_usage.total_usage;
            if let Some(system_cpu) = stats.cpu_stats.system_cpu_usage {
                let prev_total = stats.precpu_stats.cpu_usage.total_usage;
                let prev_system = stats.precpu_stats.system_cpu_usage.unwrap_or(0);
                let cpu_delta = total_usage.saturating_sub(prev_total);
                let system_delta = system_cpu.saturating_sub(prev_system);
                if system_delta > 0 {
                    let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as u64;
                    let cpu_pct = (cpu_delta as f64 / system_delta as f64) * num_cpus as f64;
                    total_cpu_nano += (cpu_pct * 1_000_000_000.0) as u64;
                }
            }

            // Memory: use memory_stats.usage minus cache for working set
            if let Some(usage) = stats.memory_stats.usage {
                let cache = stats.memory_stats.stats
                    .as_ref()
                    .map(|s| match s {
                        bollard::container::MemoryStatsStats::V1(v1) => v1.cache,
                        bollard::container::MemoryStatsStats::V2(v2) => v2.inactive_file,
                    })
                    .unwrap_or(0);
                total_memory_bytes += usage.saturating_sub(cache);
            }

            sampled += 1;
        }
    }

    // If we sampled fewer containers than exist, extrapolate
    let total_pods = pod_containers.len() as u64;
    if sampled > 0 && total_pods > sampled {
        let scale = total_pods as f64 / sampled as f64;
        total_cpu_nano = (total_cpu_nano as f64 * scale) as u64;
        total_memory_bytes = (total_memory_bytes as f64 * scale) as u64;
    }

    let cpu_millicores = total_cpu_nano / 1_000_000;
    let memory_mi = total_memory_bytes / (1024 * 1024);

    debug!("Node {} metrics: {}m CPU, {}Mi memory ({} containers sampled)",
        node_name, cpu_millicores, memory_mi, sampled);

    (format!("{}m", cpu_millicores), format!("{}Mi", memory_mi))
}

/// Get metrics for a specific node
pub async fn get_node_metrics(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<NodeMetrics>> {
    debug!("Getting node metrics: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "nodes")
        .with_api_group("metrics.k8s.io")
        .with_subresource("metrics")
        .with_name(&name);

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Check if node exists
    let node_key = build_key("nodes", None, &name);
    let _node: rusternetes_common::resources::Node = state.storage.as_ref().get(&node_key).await?;

    // Query Docker for real container stats
    let (cpu, memory) = collect_node_usage_from_docker(&name).await;
    let mut usage = BTreeMap::new();
    usage.insert("cpu".to_string(), cpu);
    usage.insert("memory".to_string(), memory);

    let metrics = NodeMetrics {
        api_version: "metrics.k8s.io/v1beta1".to_string(),
        kind: "NodeMetrics".to_string(),
        metadata: NodeMetricsMetadata {
            name,
            creation_timestamp: Some(Utc::now()),
        },
        timestamp: Utc::now(),
        window: "30s".to_string(),
        usage,
    };

    Ok(Json(metrics))
}

/// List metrics for all nodes
pub async fn list_node_metrics(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<List<NodeMetrics>>> {
    debug!("Listing node metrics");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "nodes")
        .with_api_group("metrics.k8s.io")
        .with_subresource("metrics");

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Get all nodes
    let nodes_prefix = build_prefix("nodes", None);
    let nodes: Vec<rusternetes_common::resources::Node> =
        state.storage.as_ref().list(&nodes_prefix).await?;
    let mut metrics_list = Vec::new();

    for node in nodes {
        // Query Docker for real container stats
        let (cpu, memory) = collect_node_usage_from_docker(&node.metadata.name).await;
        let mut usage = BTreeMap::new();
        usage.insert("cpu".to_string(), cpu);
        usage.insert("memory".to_string(), memory);

        let metrics = NodeMetrics {
            api_version: "metrics.k8s.io/v1beta1".to_string(),
            kind: "NodeMetrics".to_string(),
            metadata: NodeMetricsMetadata {
                name: node.metadata.name.clone(),
                creation_timestamp: Some(Utc::now()),
            },
            timestamp: Utc::now(),
            window: "30s".to_string(),
            usage,
        };
        metrics_list.push(metrics);
    }

    let list = List::new("NodeMetricsList", "metrics.k8s.io/v1beta1", metrics_list);
    Ok(Json(list))
}

/// Get metrics for a specific pod
pub async fn get_pod_metrics(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<PodMetrics>> {
    debug!("Getting pod metrics: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "pods")
        .with_api_group("metrics.k8s.io")
        .with_namespace(&namespace)
        .with_subresource("metrics")
        .with_name(&name);

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Check if pod exists
    let pod_key = build_key("pods", Some(&namespace), &name);
    let pod: rusternetes_common::resources::Pod = state.storage.as_ref().get(&pod_key).await?;

    // Return metrics based on pod spec resource requests/limits
    let mut containers = Vec::new();
    if let Some(spec) = &pod.spec {
        for container in &spec.containers {
            let mut usage = BTreeMap::new();
            let cpu = container
                .resources
                .as_ref()
                .and_then(|r| r.requests.as_ref())
                .and_then(|req| req.get("cpu"))
                .or_else(|| {
                    container
                        .resources
                        .as_ref()
                        .and_then(|r| r.limits.as_ref())
                        .and_then(|lim| lim.get("cpu"))
                })
                .cloned()
                .unwrap_or_else(|| "100m".to_string());
            let memory = container
                .resources
                .as_ref()
                .and_then(|r| r.requests.as_ref())
                .and_then(|req| req.get("memory"))
                .or_else(|| {
                    container
                        .resources
                        .as_ref()
                        .and_then(|r| r.limits.as_ref())
                        .and_then(|lim| lim.get("memory"))
                })
                .cloned()
                .unwrap_or_else(|| "128Mi".to_string());
            usage.insert("cpu".to_string(), cpu);
            usage.insert("memory".to_string(), memory);

            containers.push(ContainerMetrics {
                name: container.name.clone(),
                usage,
            });
        }
    }

    let metrics = PodMetrics {
        api_version: "metrics.k8s.io/v1beta1".to_string(),
        kind: "PodMetrics".to_string(),
        metadata: PodMetricsMetadata {
            name,
            namespace,
            creation_timestamp: Some(Utc::now()),
        },
        timestamp: Utc::now(),
        window: "30s".to_string(),
        containers,
    };

    Ok(Json(metrics))
}

/// List metrics for all pods in a namespace
pub async fn list_pod_metrics(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<List<PodMetrics>>> {
    debug!("Listing pod metrics in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "pods")
        .with_api_group("metrics.k8s.io")
        .with_namespace(&namespace)
        .with_subresource("metrics");

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Get all pods in namespace
    let pods_prefix = build_prefix("pods", Some(&namespace));
    let pods: Vec<rusternetes_common::resources::Pod> =
        state.storage.as_ref().list(&pods_prefix).await?;
    let mut metrics_list = Vec::new();

    for pod in pods {
        let mut containers = Vec::new();
        if let Some(spec) = &pod.spec {
            for container in &spec.containers {
                let mut usage = BTreeMap::new();
                let cpu = container
                    .resources
                    .as_ref()
                    .and_then(|r| r.requests.as_ref())
                    .and_then(|req| req.get("cpu"))
                    .or_else(|| {
                        container
                            .resources
                            .as_ref()
                            .and_then(|r| r.limits.as_ref())
                            .and_then(|lim| lim.get("cpu"))
                    })
                    .cloned()
                    .unwrap_or_else(|| "100m".to_string());
                let memory = container
                    .resources
                    .as_ref()
                    .and_then(|r| r.requests.as_ref())
                    .and_then(|req| req.get("memory"))
                    .or_else(|| {
                        container
                            .resources
                            .as_ref()
                            .and_then(|r| r.limits.as_ref())
                            .and_then(|lim| lim.get("memory"))
                    })
                    .cloned()
                    .unwrap_or_else(|| "128Mi".to_string());
                usage.insert("cpu".to_string(), cpu);
                usage.insert("memory".to_string(), memory);

                containers.push(ContainerMetrics {
                    name: container.name.clone(),
                    usage,
                });
            }
        }

        let metrics = PodMetrics {
            api_version: "metrics.k8s.io/v1beta1".to_string(),
            kind: "PodMetrics".to_string(),
            metadata: PodMetricsMetadata {
                name: pod.metadata.name.clone(),
                namespace: namespace.clone(),
                creation_timestamp: Some(Utc::now()),
            },
            timestamp: Utc::now(),
            window: "30s".to_string(),
            containers,
        };
        metrics_list.push(metrics);
    }

    let list = List::new("PodMetricsList", "metrics.k8s.io/v1beta1", metrics_list);
    Ok(Json(list))
}

/// List metrics for all pods across all namespaces
pub async fn list_all_pod_metrics(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<List<PodMetrics>>> {
    debug!("Listing pod metrics across all namespaces");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "pods")
        .with_api_group("metrics.k8s.io")
        .with_subresource("metrics");

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Get all namespaces first
    let ns_prefix = build_prefix("namespaces", None);
    let namespaces: Vec<rusternetes_common::resources::Namespace> =
        state.storage.as_ref().list(&ns_prefix).await?;
    let mut metrics_list = Vec::new();

    for ns in namespaces {
        // Get all pods in this namespace
        let pods_prefix = build_prefix("pods", Some(&ns.metadata.name));
        let pods: Vec<rusternetes_common::resources::Pod> =
            state.storage.as_ref().list(&pods_prefix).await?;

        for pod in pods {
            let mut containers = Vec::new();
            if let Some(spec) = &pod.spec {
                for container in &spec.containers {
                    let mut usage = BTreeMap::new();
                    let cpu = container
                        .resources
                        .as_ref()
                        .and_then(|r| r.requests.as_ref())
                        .and_then(|req| req.get("cpu"))
                        .or_else(|| {
                            container
                                .resources
                                .as_ref()
                                .and_then(|r| r.limits.as_ref())
                                .and_then(|lim| lim.get("cpu"))
                        })
                        .cloned()
                        .unwrap_or_else(|| "100m".to_string());
                    let memory = container
                        .resources
                        .as_ref()
                        .and_then(|r| r.requests.as_ref())
                        .and_then(|req| req.get("memory"))
                        .or_else(|| {
                            container
                                .resources
                                .as_ref()
                                .and_then(|r| r.limits.as_ref())
                                .and_then(|lim| lim.get("memory"))
                        })
                        .cloned()
                        .unwrap_or_else(|| "128Mi".to_string());
                    usage.insert("cpu".to_string(), cpu);
                    usage.insert("memory".to_string(), memory);

                    containers.push(ContainerMetrics {
                        name: container.name.clone(),
                        usage,
                    });
                }
            }

            let metrics = PodMetrics {
                api_version: "metrics.k8s.io/v1beta1".to_string(),
                kind: "PodMetrics".to_string(),
                metadata: PodMetricsMetadata {
                    name: pod.metadata.name.clone(),
                    namespace: ns.metadata.name.clone(),
                    creation_timestamp: Some(Utc::now()),
                },
                timestamp: Utc::now(),
                window: "30s".to_string(),
                containers,
            };
            metrics_list.push(metrics);
        }
    }

    let list = List::new("PodMetricsList", "metrics.k8s.io/v1beta1", metrics_list);
    Ok(Json(list))
}

#[cfg(test)]
#[cfg(feature = "integration-tests")] // Disable tests that require full setup
mod tests {
    use super::*;
    use crate::state::ApiServerState;
    use rusternetes_common::auth::UserInfo;
    use rusternetes_common::authz::AlwaysAllowAuthorizer;
    use rusternetes_common::resources::{Namespace, Node, NodeSpec, Pod, PodSpec};
    use rusternetes_common::storage::MemoryStorage;
    use rusternetes_common::types::ObjectMeta;

    async fn create_test_state() -> Arc<ApiServerState> {
        use rusternetes_common::auth::{BootstrapTokenManager, TokenManager};
        use rusternetes_common::observability::MetricsRegistry;
        use rusternetes_storage::memory::MemoryStorage;

        let storage = Arc::new(MemoryStorage::new());
        let token_manager = Arc::new(TokenManager::new(b"test-key"));
        let bootstrap_token_manager = Arc::new(BootstrapTokenManager::new());
        let authorizer = Arc::new(AlwaysAllowAuthorizer);
        let metrics = Arc::new(MetricsRegistry::new());

        // Create a test node
        let node = Node {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Node".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("test-node"),
            spec: Some(NodeSpec {
                pod_cidr: Some("10.244.0.0/24".to_string()),
                pod_cidrs: None,
                provider_id: None,
                taints: None,
                unschedulable: None,
            }),
            status: None,
        };
        storage
            .create("/registry/nodes/test-node", &node)
            .await
            .unwrap();

        // Create a test namespace
        let ns = Namespace::new("default");
        storage
            .create("/registry/namespaces/default", &ns)
            .await
            .unwrap();

        // Create a test pod
        let pod = Pod::new(
            "test-pod",
            PodSpec {
                containers: vec![rusternetes_common::resources::Container {
                    name: "nginx".to_string(),
                    image: "nginx:latest".to_string(),
                    command: None,
                    args: None,
                    working_dir: None,
                    ports: None,
                    env: None,
                    resources: None,
                    volume_mounts: None,
                    image_pull_policy: None,
                    liveness_probe: None,
                    readiness_probe: None,
                    startup_probe: None,
                    security_context: None,
                    restart_policy: None,
                }],
                init_containers: None,
                ephemeral_containers: None,
                volumes: None,
                restart_policy: Some("Always".to_string()),
                node_name: None,
                node_selector: None,
                service_account_name: None,
                service_account: None,
                hostname: None,
                subdomain: None,
                host_network: None,
                host_pid: None,
                host_ipc: None,
                affinity: None,
                tolerations: None,
                priority: None,
                priority_class_name: None,
                automount_service_account_token: None,
                topology_spread_constraints: None,
                overhead: None,
                scheduler_name: None,
                resource_claims: None,
            },
        );
        storage
            .create("/registry/pods/default/test-pod", &pod)
            .await
            .unwrap();

        Arc::new(ApiServerState::new(
            storage,
            token_manager,
            bootstrap_token_manager,
            authorizer,
            metrics,
            true, // skip_auth for tests
            None, // ca_cert_pem
        ))
    }

    #[tokio::test]
    async fn test_get_node_metrics() {
        let state = create_test_state().await;
        let auth_ctx = AuthContext {
            user: UserInfo {
                username: "test-user".to_string(),
                uid: "test-uid".to_string(),
                groups: vec![],
                extra: Default::default(),
            },
        };

        let result = get_node_metrics(
            State(state),
            Extension(auth_ctx),
            Path("test-node".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let metrics = result.unwrap().0;
        assert_eq!(metrics.metadata.name, "test-node");
        assert!(metrics.usage.contains_key("cpu"));
        assert!(metrics.usage.contains_key("memory"));
    }

    #[tokio::test]
    async fn test_list_node_metrics() {
        let state = create_test_state().await;
        let auth_ctx = AuthContext {
            user: UserInfo {
                username: "test-user".to_string(),
                uid: "test-uid".to_string(),
                groups: vec![],
                extra: Default::default(),
            },
        };

        let result = list_node_metrics(State(state), Extension(auth_ctx)).await;

        assert!(result.is_ok());
        let list = result.unwrap().0;
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].metadata.name, "test-node");
    }

    #[tokio::test]
    async fn test_get_pod_metrics() {
        let state = create_test_state().await;
        let auth_ctx = AuthContext {
            user: UserInfo {
                username: "test-user".to_string(),
                uid: "test-uid".to_string(),
                groups: vec![],
                extra: Default::default(),
            },
        };

        let result = get_pod_metrics(
            State(state),
            Extension(auth_ctx),
            Path(("default".to_string(), "test-pod".to_string())),
        )
        .await;

        assert!(result.is_ok());
        let metrics = result.unwrap().0;
        assert_eq!(metrics.metadata.name, "test-pod");
        assert_eq!(metrics.metadata.namespace, "default");
        assert_eq!(metrics.containers.len(), 1);
        assert_eq!(metrics.containers[0].name, "nginx");
    }

    #[tokio::test]
    async fn test_list_pod_metrics() {
        let state = create_test_state().await;
        let auth_ctx = AuthContext {
            user: UserInfo {
                username: "test-user".to_string(),
                uid: "test-uid".to_string(),
                groups: vec![],
                extra: Default::default(),
            },
        };

        let result = list_pod_metrics(
            State(state),
            Extension(auth_ctx),
            Path("default".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let list = result.unwrap().0;
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].metadata.name, "test-pod");
    }
}

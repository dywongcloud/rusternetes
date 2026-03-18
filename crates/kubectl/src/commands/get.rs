use crate::client::{ApiClient, GetError};
use anyhow::{Context, Result};
use chrono::Utc;
use rusternetes_common::resources::{
    ClusterRole, ClusterRoleBinding, ConfigMap, CronJob, CustomResourceDefinition, DaemonSet,
    Deployment, Endpoints, HorizontalPodAutoscaler, Ingress, Job, LimitRange, Namespace, Node,
    PersistentVolume, PersistentVolumeClaim, Pod, PodDisruptionBudget, PriorityClass,
    ResourceQuota, Role, RoleBinding, Secret, Service, ServiceAccount, StatefulSet, StorageClass,
    VerticalPodAutoscaler, VolumeSnapshot, VolumeSnapshotClass,
};
use serde::Serialize;

// Helper to convert GetError to anyhow::Error
fn map_get_error(err: GetError) -> anyhow::Error {
    match err {
        GetError::NotFound => anyhow::anyhow!("Resource not found"),
        GetError::Other(e) => e,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OutputFormat {
    Table,
    Json,
    Yaml,
    Wide,
    Name,
    JsonPath(String),
}

impl OutputFormat {
    pub(crate) fn from_str(s: &str) -> Result<Self> {
        match s {
            "json" => Ok(Self::Json),
            "yaml" => Ok(Self::Yaml),
            "wide" => Ok(Self::Wide),
            "name" => Ok(Self::Name),
            _ if s.starts_with("jsonpath=") => Ok(Self::JsonPath(s[9..].to_string())),
            _ if s.starts_with("jsonpath-file=") => {
                let path = &s[14..];
                let expr = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read jsonpath file: {}", path))?;
                Ok(Self::JsonPath(expr.trim().to_string()))
            }
            _ => anyhow::bail!(
                "Unknown output format: {}. Supported formats: json, yaml, wide, name, jsonpath=<expr>",
                s
            ),
        }
    }
}

pub(crate) fn format_output<T: Serialize>(resource: &T, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(resource)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(resource)?);
        }
        OutputFormat::JsonPath(expr) => {
            let value = serde_json::to_value(resource)?;
            let result = evaluate_jsonpath(&value, expr)?;
            print!("{}", result);
        }
        OutputFormat::Name => {
            // Print kind/name format
            let value = serde_json::to_value(resource)?;
            let kind = value
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let name = value
                .pointer("/metadata/name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!("{}/{}", kind.to_lowercase(), name);
        }
        _ => {
            // For table and wide, just use JSON for now
            println!("{}", serde_json::to_string_pretty(resource)?);
        }
    }
    Ok(())
}

/// Evaluate a jsonpath expression against a JSON value.
/// Supports basic dotted paths, array indexing, and filter expressions.
fn evaluate_jsonpath(value: &serde_json::Value, expr: &str) -> Result<String> {
    // Strip leading { and trailing } if present (kubectl jsonpath format)
    let expr = expr.trim();
    let expr = expr.strip_prefix('{').unwrap_or(expr);
    let expr = expr.strip_suffix('}').unwrap_or(expr);

    // Handle multiple expressions joined by whitespace (e.g., {.foo}{"\n"}{.bar})
    // For simplicity, evaluate the full expr as a single path
    let result = resolve_path(value, expr)?;
    Ok(result)
}

fn resolve_path(value: &serde_json::Value, path: &str) -> Result<String> {
    // Handle string literals like "\n", "\t"
    if path.starts_with('"') && path.ends_with('"') {
        let s = &path[1..path.len() - 1];
        return Ok(s.replace("\\n", "\n").replace("\\t", "\t"));
    }

    // Strip leading dot
    let path = path.strip_prefix('.').unwrap_or(path);

    if path.is_empty() {
        return format_value(value);
    }

    // Split on first dot or bracket
    let mut current = value;
    let mut remaining = path;

    while !remaining.is_empty() {
        if remaining.starts_with('[') {
            // Array index or filter
            let end = remaining.find(']').unwrap_or(remaining.len());
            let idx_str = &remaining[1..end];
            remaining = remaining
                .get(end + 1..)
                .unwrap_or("")
                .trim_start_matches('.');

            if let Ok(idx) = idx_str.parse::<usize>() {
                match current.get(idx) {
                    Some(v) => current = v,
                    None => return Ok(String::new()),
                }
            } else if idx_str.starts_with('?') {
                // Filter expression like [?(@.type=="Ready")]
                // For now just return empty string for unsupported filters
                return Ok(String::new());
            } else {
                return Ok(String::new());
            }
        } else {
            // Field access
            let (key, rest) = match remaining.find(|c| c == '.' || c == '[') {
                Some(i) => {
                    let (k, r) = remaining.split_at(i);
                    (k, r.trim_start_matches('.'))
                }
                None => (remaining, ""),
            };
            remaining = rest;
            match current.get(key) {
                Some(v) => current = v,
                None => return Ok(String::new()),
            }
        }
    }

    format_value(current)
}

fn format_value(value: &serde_json::Value) -> Result<String> {
    match value {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Null => Ok(String::new()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Array(arr) => {
            let parts: Result<Vec<_>> = arr.iter().map(format_value).collect();
            Ok(parts?.join(" "))
        }
        serde_json::Value::Object(_) => Ok(serde_json::to_string(value)?),
    }
}

/// Enhanced execute with all new parameters
pub async fn execute_enhanced(
    client: &ApiClient,
    resource_type: &str,
    name: Option<&str>,
    namespace: Option<&str>,
    all_namespaces: bool,
    output: Option<&str>,
    no_headers: bool,
    selector: Option<&str>,
    field_selector: Option<&str>,
    watch: bool,
    show_labels: bool,
) -> Result<()> {
    if watch {
        println!("Watch mode not yet implemented");
        return Ok(());
    }
    if field_selector.is_some() {
        println!("Note: Field selector filtering not yet fully implemented");
    }

    // Build query string for label selector and field selector
    let mut query_params = Vec::new();
    if let Some(sel) = selector {
        query_params.push(format!("labelSelector={}", urlencoding::encode(sel)));
    }
    if let Some(fs) = field_selector {
        query_params.push(format!("fieldSelector={}", urlencoding::encode(fs)));
    }

    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    // Delegate to the original execute with query string
    execute_with_query(
        client,
        resource_type,
        name,
        namespace,
        output,
        no_headers,
        &query_string,
        show_labels,
    )
    .await
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' | '=' | ',' | '!' => {
                    c.to_string()
                }
                ' ' => "+".to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
}

pub async fn execute_with_query(
    client: &ApiClient,
    resource_type: &str,
    name: Option<&str>,
    namespace: Option<&str>,
    output: Option<&str>,
    no_headers: bool,
    query: &str,
    _show_labels: bool,
) -> Result<()> {
    let default_namespace = "default";
    let ns = namespace.unwrap_or(default_namespace);
    let format = output
        .map(OutputFormat::from_str)
        .transpose()?
        .unwrap_or(OutputFormat::Table);

    // Helper macro to reduce code duplication
    macro_rules! get_resources {
        ($path:expr, $type:ty, $print_fn:expr) => {{
            let full_path = format!("{}{}", $path, query);
            if name.is_some() {
                let resource: $type = client.get(&full_path).await.map_err(map_get_error)?;
                format_output(&resource, &format)?;
            } else {
                let resources: Vec<$type> =
                    client.get_list(&full_path).await.map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => $print_fn(&resources, no_headers),
                    _ => format_output(&resources, &format)?,
                }
            }
        }};
    }

    match resource_type {
        "pod" | "pods" => {
            get_resources!(
                format!(
                    "/api/v1/namespaces/{}/pods{}",
                    ns,
                    if name.is_some() {
                        format!("/{}", name.unwrap())
                    } else {
                        String::new()
                    }
                ),
                Pod,
                print_pods
            );
        }
        "service" | "services" | "svc" => {
            get_resources!(
                format!(
                    "/api/v1/namespaces/{}/services{}",
                    ns,
                    if name.is_some() {
                        format!("/{}", name.unwrap())
                    } else {
                        String::new()
                    }
                ),
                Service,
                print_services
            );
        }
        "deployment" | "deployments" | "deploy" => {
            get_resources!(
                format!(
                    "/apis/apps/v1/namespaces/{}/deployments{}",
                    ns,
                    if name.is_some() {
                        format!("/{}", name.unwrap())
                    } else {
                        String::new()
                    }
                ),
                Deployment,
                print_deployments
            );
        }
        "job" | "jobs" => {
            get_resources!(
                format!(
                    "/apis/batch/v1/namespaces/{}/jobs{}",
                    ns,
                    if name.is_some() {
                        format!("/{}", name.unwrap())
                    } else {
                        String::new()
                    }
                ),
                Job,
                print_jobs
            );
        }
        "cronjob" | "cronjobs" | "cj" => {
            get_resources!(
                format!(
                    "/apis/batch/v1/namespaces/{}/cronjobs{}",
                    ns,
                    if name.is_some() {
                        format!("/{}", name.unwrap())
                    } else {
                        String::new()
                    }
                ),
                CronJob,
                print_cronjobs
            );
        }
        "node" | "nodes" => {
            get_resources!(
                format!(
                    "/api/v1/nodes{}",
                    if name.is_some() {
                        format!("/{}", name.unwrap())
                    } else {
                        String::new()
                    }
                ),
                Node,
                print_nodes
            );
        }
        "namespace" | "namespaces" | "ns" => {
            get_resources!(
                format!(
                    "/api/v1/namespaces{}",
                    if name.is_some() {
                        format!("/{}", name.unwrap())
                    } else {
                        String::new()
                    }
                ),
                Namespace,
                print_namespaces
            );
        }
        _ => {
            // Fall back to original execute for other resource types
            return execute(client, resource_type, name, namespace, output, no_headers).await;
        }
    }

    Ok(())
}

pub async fn execute(
    client: &ApiClient,
    resource_type: &str,
    name: Option<&str>,
    namespace: Option<&str>,
    output: Option<&str>,
    no_headers: bool,
) -> Result<()> {
    let default_namespace = "default";
    let ns = namespace.unwrap_or(default_namespace);
    let format = output
        .map(OutputFormat::from_str)
        .transpose()?
        .unwrap_or(OutputFormat::Table);

    match resource_type {
        "all" => {
            // Get all common resources
            println!("Fetching all resources in namespace {}...\n", ns);

            // Pods
            let pods: Vec<Pod> = client
                .get_list(&format!("/api/v1/namespaces/{}/pods", ns))
                .await
                .unwrap_or_default();
            if !pods.is_empty() {
                print_pods(&pods, no_headers);
                println!();
            }

            // Services
            let services: Vec<Service> = client
                .get_list(&format!("/api/v1/namespaces/{}/services", ns))
                .await
                .unwrap_or_default();
            if !services.is_empty() {
                print_services(&services, no_headers);
                println!();
            }

            // Deployments
            let deployments: Vec<Deployment> = client
                .get_list(&format!("/apis/apps/v1/namespaces/{}/deployments", ns))
                .await
                .unwrap_or_default();
            if !deployments.is_empty() {
                print_deployments(&deployments, no_headers);
                println!();
            }

            // StatefulSets
            let statefulsets: Vec<StatefulSet> = client
                .get_list(&format!("/apis/apps/v1/namespaces/{}/statefulsets", ns))
                .await
                .unwrap_or_default();
            if !statefulsets.is_empty() {
                println!("{:<30} {:<15}", "NAME", "READY");
                for sts in &statefulsets {
                    println!("{:<30} {:<15}", sts.metadata.name, "statefulset");
                }
                println!();
            }

            // DaemonSets
            let daemonsets: Vec<DaemonSet> = client
                .get_list(&format!("/apis/apps/v1/namespaces/{}/daemonsets", ns))
                .await
                .unwrap_or_default();
            if !daemonsets.is_empty() {
                println!("{:<30} {:<15}", "NAME", "TYPE");
                for ds in &daemonsets {
                    println!("{:<30} {:<15}", ds.metadata.name, "daemonset");
                }
                println!();
            }

            // Jobs
            let jobs: Vec<Job> = client
                .get_list(&format!("/apis/batch/v1/namespaces/{}/jobs", ns))
                .await
                .unwrap_or_default();
            if !jobs.is_empty() {
                print_jobs(&jobs, no_headers);
                println!();
            }

            // CronJobs
            let cronjobs: Vec<CronJob> = client
                .get_list(&format!("/apis/batch/v1/namespaces/{}/cronjobs", ns))
                .await
                .unwrap_or_default();
            if !cronjobs.is_empty() {
                print_cronjobs(&cronjobs, no_headers);
            }
        }
        "pod" | "pods" => {
            if let Some(name) = name {
                let pod: Pod = client
                    .get(&format!("/api/v1/namespaces/{}/pods/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&pod, &format)?;
            } else {
                let pods: Vec<Pod> = client
                    .get_list(&format!("/api/v1/namespaces/{}/pods", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_pods(&pods, no_headers),
                    _ => format_output(&pods, &format)?,
                }
            }
        }
        "service" | "services" | "svc" => {
            if let Some(name) = name {
                let service: Service = client
                    .get(&format!("/api/v1/namespaces/{}/services/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&service, &format)?;
            } else {
                let services: Vec<Service> = client
                    .get_list(&format!("/api/v1/namespaces/{}/services", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => {
                        print_services(&services, no_headers)
                    }
                    _ => format_output(&services, &format)?,
                }
            }
        }
        "deployment" | "deployments" | "deploy" => {
            if let Some(name) = name {
                let deployment: Deployment = client
                    .get(&format!(
                        "/apis/apps/v1/namespaces/{}/deployments/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&deployment, &format)?;
            } else {
                let deployments: Vec<Deployment> = client
                    .get_list(&format!("/apis/apps/v1/namespaces/{}/deployments", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => {
                        print_deployments(&deployments, no_headers)
                    }
                    _ => format_output(&deployments, &format)?,
                }
            }
        }
        "statefulset" | "statefulsets" | "sts" => {
            if let Some(name) = name {
                let statefulset: StatefulSet = client
                    .get(&format!(
                        "/apis/apps/v1/namespaces/{}/statefulsets/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&statefulset, &format)?;
            } else {
                let statefulsets: Vec<StatefulSet> = client
                    .get_list(&format!("/apis/apps/v1/namespaces/{}/statefulsets", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&statefulsets, &format)?;
            }
        }
        "daemonset" | "daemonsets" | "ds" => {
            if let Some(name) = name {
                let daemonset: DaemonSet = client
                    .get(&format!(
                        "/apis/apps/v1/namespaces/{}/daemonsets/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&daemonset, &format)?;
            } else {
                let daemonsets: Vec<DaemonSet> = client
                    .get_list(&format!("/apis/apps/v1/namespaces/{}/daemonsets", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&daemonsets, &format)?;
            }
        }
        "job" | "jobs" => {
            if let Some(name) = name {
                let job: Job = client
                    .get(&format!("/apis/batch/v1/namespaces/{}/jobs/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&job, &format)?;
            } else {
                let jobs: Vec<Job> = client
                    .get_list(&format!("/apis/batch/v1/namespaces/{}/jobs", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_jobs(&jobs, no_headers),
                    _ => format_output(&jobs, &format)?,
                }
            }
        }
        "cronjob" | "cronjobs" | "cj" => {
            if let Some(name) = name {
                let cronjob: CronJob = client
                    .get(&format!(
                        "/apis/batch/v1/namespaces/{}/cronjobs/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&cronjob, &format)?;
            } else {
                let cronjobs: Vec<CronJob> = client
                    .get_list(&format!("/apis/batch/v1/namespaces/{}/cronjobs", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => {
                        print_cronjobs(&cronjobs, no_headers)
                    }
                    _ => format_output(&cronjobs, &format)?,
                }
            }
        }
        "node" | "nodes" => {
            if let Some(name) = name {
                let node: Node = client
                    .get(&format!("/api/v1/nodes/{}", name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&node, &format)?;
            } else {
                let nodes: Vec<Node> = client
                    .get_list("/api/v1/nodes")
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_nodes(&nodes, no_headers),
                    _ => format_output(&nodes, &format)?,
                }
            }
        }
        "namespace" | "namespaces" | "ns" => {
            if let Some(name) = name {
                let namespace: Namespace = client
                    .get(&format!("/api/v1/namespaces/{}", name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&namespace, &format)?;
            } else {
                let namespaces: Vec<Namespace> = client
                    .get_list("/api/v1/namespaces")
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => {
                        print_namespaces(&namespaces, no_headers)
                    }
                    _ => format_output(&namespaces, &format)?,
                }
            }
        }
        "persistentvolume" | "persistentvolumes" | "pv" => {
            if let Some(name) = name {
                let pv: PersistentVolume = client
                    .get(&format!("/api/v1/persistentvolumes/{}", name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&pv, &format)?;
            } else {
                let pvs: Vec<PersistentVolume> = client
                    .get_list("/api/v1/persistentvolumes")
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_pvs(&pvs, no_headers),
                    _ => format_output(&pvs, &format)?,
                }
            }
        }
        "persistentvolumeclaim" | "persistentvolumeclaims" | "pvc" => {
            if let Some(name) = name {
                let pvc: PersistentVolumeClaim = client
                    .get(&format!(
                        "/api/v1/namespaces/{}/persistentvolumeclaims/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&pvc, &format)?;
            } else {
                let pvcs: Vec<PersistentVolumeClaim> = client
                    .get_list(&format!("/api/v1/namespaces/{}/persistentvolumeclaims", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_pvcs(&pvcs, no_headers),
                    _ => format_output(&pvcs, &format)?,
                }
            }
        }
        "storageclass" | "storageclasses" | "sc" => {
            if let Some(name) = name {
                let sc: StorageClass = client
                    .get(&format!("/apis/storage.k8s.io/v1/storageclasses/{}", name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&sc, &format)?;
            } else {
                let scs: Vec<StorageClass> = client
                    .get_list("/apis/storage.k8s.io/v1/storageclasses")
                    .await
                    .map_err(map_get_error)?;
                format_output(&scs, &format)?;
            }
        }
        "volumesnapshot" | "volumesnapshots" | "vs" => {
            if let Some(name) = name {
                let vs: VolumeSnapshot = client
                    .get(&format!(
                        "/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&vs, &format)?;
            } else {
                let vss: Vec<VolumeSnapshot> = client
                    .get_list(&format!(
                        "/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots",
                        ns
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&vss, &format)?;
            }
        }
        "volumesnapshotclass" | "volumesnapshotclasses" | "vsc" => {
            if let Some(name) = name {
                let vsc: VolumeSnapshotClass = client
                    .get(&format!(
                        "/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/{}",
                        name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&vsc, &format)?;
            } else {
                let vscs: Vec<VolumeSnapshotClass> = client
                    .get_list("/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses")
                    .await
                    .map_err(map_get_error)?;
                format_output(&vscs, &format)?;
            }
        }
        "endpoints" | "ep" => {
            if let Some(name) = name {
                let ep: Endpoints = client
                    .get(&format!("/api/v1/namespaces/{}/endpoints/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&ep, &format)?;
            } else {
                let eps: Vec<Endpoints> = client
                    .get_list(&format!("/api/v1/namespaces/{}/endpoints", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&eps, &format)?;
            }
        }
        "configmap" | "configmaps" | "cm" => {
            if let Some(name) = name {
                let cm: ConfigMap = client
                    .get(&format!("/api/v1/namespaces/{}/configmaps/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&cm, &format)?;
            } else {
                let cms: Vec<ConfigMap> = client
                    .get_list(&format!("/api/v1/namespaces/{}/configmaps", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&cms, &format)?;
            }
        }
        "secret" | "secrets" => {
            if let Some(name) = name {
                let secret: Secret = client
                    .get(&format!("/api/v1/namespaces/{}/secrets/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&secret, &format)?;
            } else {
                let secrets: Vec<Secret> = client
                    .get_list(&format!("/api/v1/namespaces/{}/secrets", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&secrets, &format)?;
            }
        }
        "ingress" | "ingresses" | "ing" => {
            if let Some(name) = name {
                let ing: Ingress = client
                    .get(&format!(
                        "/apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&ing, &format)?;
            } else {
                let ings: Vec<Ingress> = client
                    .get_list(&format!(
                        "/apis/networking.k8s.io/v1/namespaces/{}/ingresses",
                        ns
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&ings, &format)?;
            }
        }
        "serviceaccount" | "serviceaccounts" | "sa" => {
            if let Some(name) = name {
                let sa: ServiceAccount = client
                    .get(&format!(
                        "/api/v1/namespaces/{}/serviceaccounts/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&sa, &format)?;
            } else {
                let sas: Vec<ServiceAccount> = client
                    .get_list(&format!("/api/v1/namespaces/{}/serviceaccounts", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&sas, &format)?;
            }
        }
        "role" | "roles" => {
            if let Some(name) = name {
                let role: Role = client
                    .get(&format!(
                        "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&role, &format)?;
            } else {
                let roles: Vec<Role> = client
                    .get_list(&format!(
                        "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles",
                        ns
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&roles, &format)?;
            }
        }
        "rolebinding" | "rolebindings" => {
            if let Some(name) = name {
                let rb: RoleBinding = client
                    .get(&format!(
                        "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&rb, &format)?;
            } else {
                let rbs: Vec<RoleBinding> = client
                    .get_list(&format!(
                        "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings",
                        ns
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&rbs, &format)?;
            }
        }
        "clusterrole" | "clusterroles" => {
            if let Some(name) = name {
                let cr: ClusterRole = client
                    .get(&format!(
                        "/apis/rbac.authorization.k8s.io/v1/clusterroles/{}",
                        name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&cr, &format)?;
            } else {
                let crs: Vec<ClusterRole> = client
                    .get_list("/apis/rbac.authorization.k8s.io/v1/clusterroles")
                    .await
                    .map_err(map_get_error)?;
                format_output(&crs, &format)?;
            }
        }
        "clusterrolebinding" | "clusterrolebindings" => {
            if let Some(name) = name {
                let crb: ClusterRoleBinding = client
                    .get(&format!(
                        "/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}",
                        name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&crb, &format)?;
            } else {
                let crbs: Vec<ClusterRoleBinding> = client
                    .get_list("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings")
                    .await
                    .map_err(map_get_error)?;
                format_output(&crbs, &format)?;
            }
        }
        "resourcequota" | "resourcequotas" | "quota" => {
            if let Some(name) = name {
                let rq: ResourceQuota = client
                    .get(&format!(
                        "/api/v1/namespaces/{}/resourcequotas/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&rq, &format)?;
            } else {
                let rqs: Vec<ResourceQuota> = client
                    .get_list(&format!("/api/v1/namespaces/{}/resourcequotas", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&rqs, &format)?;
            }
        }
        "limitrange" | "limitranges" | "limits" => {
            if let Some(name) = name {
                let lr: LimitRange = client
                    .get(&format!("/api/v1/namespaces/{}/limitranges/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&lr, &format)?;
            } else {
                let lrs: Vec<LimitRange> = client
                    .get_list(&format!("/api/v1/namespaces/{}/limitranges", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&lrs, &format)?;
            }
        }
        "priorityclass" | "priorityclasses" | "pc" => {
            if let Some(name) = name {
                let pc: PriorityClass = client
                    .get(&format!(
                        "/apis/scheduling.k8s.io/v1/priorityclasses/{}",
                        name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&pc, &format)?;
            } else {
                let pcs: Vec<PriorityClass> = client
                    .get_list("/apis/scheduling.k8s.io/v1/priorityclasses")
                    .await
                    .map_err(map_get_error)?;
                format_output(&pcs, &format)?;
            }
        }
        "customresourcedefinition" | "customresourcedefinitions" | "crd" | "crds" => {
            if let Some(name) = name {
                let crd: CustomResourceDefinition = client
                    .get(&format!(
                        "/apis/apiextensions.k8s.io/v1/customresourcedefinitions/{}",
                        name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&crd, &format)?;
            } else {
                let crds: Vec<CustomResourceDefinition> = client
                    .get_list("/apis/apiextensions.k8s.io/v1/customresourcedefinitions")
                    .await
                    .map_err(map_get_error)?;
                format_output(&crds, &format)?;
            }
        }
        "poddisruptionbudget" | "poddisruptionbudgets" | "pdb" => {
            if let Some(name) = name {
                let pdb: PodDisruptionBudget = client
                    .get(&format!(
                        "/apis/policy/v1/namespaces/{}/poddisruptionbudgets/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&pdb, &format)?;
            } else {
                let pdbs: Vec<PodDisruptionBudget> = client
                    .get_list(&format!(
                        "/apis/policy/v1/namespaces/{}/poddisruptionbudgets",
                        ns
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&pdbs, &format)?;
            }
        }
        "horizontalpodautoscaler" | "horizontalpodautoscalers" | "hpa" => {
            if let Some(name) = name {
                let hpa: HorizontalPodAutoscaler = client
                    .get(&format!(
                        "/apis/autoscaling/v2/namespaces/{}/horizontalpodautoscalers/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&hpa, &format)?;
            } else {
                let hpas: Vec<HorizontalPodAutoscaler> = client
                    .get_list(&format!(
                        "/apis/autoscaling/v2/namespaces/{}/horizontalpodautoscalers",
                        ns
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&hpas, &format)?;
            }
        }
        "verticalpodautoscaler" | "verticalpodautoscalers" | "vpa" => {
            if let Some(name) = name {
                let vpa: VerticalPodAutoscaler = client
                    .get(&format!(
                        "/apis/autoscaling.k8s.io/v1/namespaces/{}/verticalpodautoscalers/{}",
                        ns, name
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&vpa, &format)?;
            } else {
                let vpas: Vec<VerticalPodAutoscaler> = client
                    .get_list(&format!(
                        "/apis/autoscaling.k8s.io/v1/namespaces/{}/verticalpodautoscalers",
                        ns
                    ))
                    .await
                    .map_err(map_get_error)?;
                format_output(&vpas, &format)?;
            }
        }
        _ => anyhow::bail!("Unknown resource type: {}", resource_type),
    }

    Ok(())
}

fn print_pods(pods: &[Pod], no_headers: bool) {
    if !no_headers {
        println!("{:<30} {:<15} {:<15}", "NAME", "STATUS", "NODE");
    }
    for pod in pods {
        let status = pod
            .status
            .as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        let node = pod
            .spec
            .as_ref()
            .and_then(|s| s.node_name.as_ref())
            .map(|n| n.as_str())
            .unwrap_or("<none>");
        println!("{:<30} {:<15} {:<15}", pod.metadata.name, status, node);
    }
}

fn print_services(services: &[Service], no_headers: bool) {
    if !no_headers {
        println!("{:<30} {:<20} {:<10}", "NAME", "CLUSTER-IP", "PORTS");
    }
    for service in services {
        let cluster_ip = service
            .spec
            .cluster_ip
            .as_ref()
            .map(|ip| ip.as_str())
            .unwrap_or("<none>");
        let ports = service
            .spec
            .ports
            .iter()
            .map(|p| p.port.to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{:<30} {:<20} {:<10}",
            service.metadata.name, cluster_ip, ports
        );
    }
}

fn format_duration(duration: chrono::Duration) -> String {
    let days = duration.num_days();
    if days > 0 {
        return format!("{}d", days);
    }
    let hours = duration.num_hours();
    if hours > 0 {
        return format!("{}h", hours);
    }
    let minutes = duration.num_minutes();
    if minutes > 0 {
        return format!("{}m", minutes);
    }
    let seconds = duration.num_seconds();
    format!("{}s", seconds)
}

fn print_deployments(deployments: &[Deployment], no_headers: bool) {
    if !no_headers {
        println!(
            "{:<30} {:<15} {:<15} {:<15} {:<10}",
            "NAME", "READY", "UP-TO-DATE", "AVAILABLE", "AGE"
        );
    }
    for deployment in deployments {
        let desired = deployment.spec.replicas.unwrap_or(1);
        let ready = deployment
            .status
            .as_ref()
            .and_then(|s| s.ready_replicas)
            .unwrap_or(0);
        let updated = deployment
            .status
            .as_ref()
            .and_then(|s| s.updated_replicas)
            .unwrap_or(0);
        let available = deployment
            .status
            .as_ref()
            .and_then(|s| s.available_replicas)
            .unwrap_or(0);
        let age = deployment
            .metadata
            .creation_timestamp
            .map(|ts| format_duration(Utc::now().signed_duration_since(ts)))
            .unwrap_or_else(|| "<unknown>".to_string());
        println!(
            "{:<30} {:<15} {:<15} {:<15} {:<10}",
            deployment.metadata.name,
            format!("{}/{}", ready, desired),
            updated,
            available,
            age
        );
    }
}

fn print_nodes(nodes: &[Node], no_headers: bool) {
    if !no_headers {
        println!("{:<30} {:<15}", "NAME", "STATUS");
    }
    for node in nodes {
        let status = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .and_then(|c| c.iter().find(|cond| cond.condition_type == "Ready"))
            .map(|c| c.status.as_str())
            .unwrap_or("Unknown");
        println!("{:<30} {:<15}", node.metadata.name, status);
    }
}

fn print_namespaces(namespaces: &[Namespace], no_headers: bool) {
    if !no_headers {
        println!("{:<30} {:<15}", "NAME", "STATUS");
    }
    for namespace in namespaces {
        let status = namespace
            .status
            .as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        println!("{:<30} {:<15}", namespace.metadata.name, status);
    }
}

fn print_pvs(pvs: &[PersistentVolume], no_headers: bool) {
    if !no_headers {
        println!(
            "{:<30} {:<15} {:<20} {:<15}",
            "NAME", "CAPACITY", "ACCESS MODES", "STATUS"
        );
    }
    for pv in pvs {
        let capacity = pv
            .spec
            .capacity
            .get("storage")
            .map(|s| s.as_str())
            .unwrap_or("<none>");
        let access_modes = pv
            .spec
            .access_modes
            .iter()
            .map(|m| format!("{:?}", m))
            .collect::<Vec<_>>()
            .join(",");
        let status = pv
            .status
            .as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        println!(
            "{:<30} {:<15} {:<20} {:<15}",
            pv.metadata.name, capacity, access_modes, status
        );
    }
}

fn print_pvcs(pvcs: &[PersistentVolumeClaim], no_headers: bool) {
    if !no_headers {
        println!(
            "{:<30} {:<15} {:<20} {:<20} {:<15}",
            "NAME", "STATUS", "VOLUME", "CAPACITY", "ACCESS MODES"
        );
    }
    for pvc in pvcs {
        let status = pvc
            .status
            .as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        let volume = pvc.spec.volume_name.as_deref().unwrap_or("<none>");
        let capacity = pvc
            .status
            .as_ref()
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("storage"))
            .map(|s| s.as_str())
            .unwrap_or("<none>");
        let access_modes = pvc
            .spec
            .access_modes
            .iter()
            .map(|m| format!("{:?}", m))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{:<30} {:<15} {:<20} {:<20} {:<15}",
            pvc.metadata.name, status, volume, capacity, access_modes
        );
    }
}

fn print_jobs(jobs: &[Job], no_headers: bool) {
    if !no_headers {
        println!("{:<30} {:<15} {:<10}", "NAME", "COMPLETIONS", "AGE");
    }
    for job in jobs {
        let completions = job.spec.completions.unwrap_or(1);
        let succeeded = job.status.as_ref().and_then(|s| s.succeeded).unwrap_or(0);
        let age = job
            .metadata
            .creation_timestamp
            .map(|ts| format_duration(Utc::now().signed_duration_since(ts)))
            .unwrap_or_else(|| "<unknown>".to_string());
        println!(
            "{:<30} {:<15} {:<10}",
            job.metadata.name,
            format!("{}/{}", succeeded, completions),
            age
        );
    }
}

fn print_cronjobs(cronjobs: &[CronJob], no_headers: bool) {
    if !no_headers {
        println!(
            "{:<30} {:<20} {:<10} {:<20} {:<10}",
            "NAME", "SCHEDULE", "SUSPEND", "ACTIVE", "LAST SCHEDULE"
        );
    }
    for cronjob in cronjobs {
        let schedule = &cronjob.spec.schedule;
        let suspend = cronjob.spec.suspend.unwrap_or(false);
        let active = cronjob
            .status
            .as_ref()
            .and_then(|s| s.active.as_ref())
            .map(|a| a.len())
            .unwrap_or(0);
        let last_schedule = cronjob
            .status
            .as_ref()
            .and_then(|s| s.last_schedule_time)
            .map(|ts| format_duration(Utc::now().signed_duration_since(ts)))
            .unwrap_or_else(|| "<none>".to_string());
        println!(
            "{:<30} {:<20} {:<10} {:<20} {:<10}",
            cronjob.metadata.name, schedule, suspend, active, last_schedule
        );
    }
}

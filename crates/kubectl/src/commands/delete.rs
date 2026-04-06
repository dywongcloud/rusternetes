use crate::client::ApiClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::io::{self, Read};

/// Cascade strategy for delete operations, matching Kubernetes propagation policies.
#[derive(Debug, Clone, PartialEq)]
pub enum CascadeStrategy {
    /// Delete dependents in the background (default).
    Background,
    /// Block until all dependents are deleted.
    Foreground,
    /// Do not delete dependents.
    Orphan,
}

impl CascadeStrategy {
    /// Parse from CLI string value.
    pub fn from_str_value(s: &str) -> Result<Self> {
        match s {
            "background" => Ok(CascadeStrategy::Background),
            "foreground" => Ok(CascadeStrategy::Foreground),
            "orphan" => Ok(CascadeStrategy::Orphan),
            _ => anyhow::bail!(
                "Invalid cascade value '{}'. Must be one of: background, foreground, orphan",
                s
            ),
        }
    }

    /// Return the Kubernetes propagation policy string for the API.
    pub fn propagation_policy(&self) -> &str {
        match self {
            CascadeStrategy::Background => "Background",
            CascadeStrategy::Foreground => "Foreground",
            CascadeStrategy::Orphan => "Orphan",
        }
    }
}

/// Options controlling delete behavior, passed through to the API server.
#[derive(Debug, Clone)]
pub struct DeleteOptions {
    /// Grace period in seconds. None means use the resource default.
    pub grace_period: Option<i64>,
    /// Force deletion (sets grace_period=0, cascade=Background).
    pub force: bool,
    /// Cascade strategy (foreground, background, orphan).
    pub cascade: CascadeStrategy,
    /// Delete all resources of the type in the namespace.
    pub delete_all: bool,
    /// Server-side dry run — no changes are persisted.
    pub dry_run: bool,
    /// Wait for resources to be fully deleted before returning.
    pub wait: bool,
    /// Output format. When "name", prints `resource/name` only.
    pub output: Option<String>,
}

impl Default for DeleteOptions {
    fn default() -> Self {
        Self {
            grace_period: None,
            force: false,
            cascade: CascadeStrategy::Background,
            delete_all: false,
            dry_run: false,
            wait: false,
            output: None,
        }
    }
}

impl DeleteOptions {
    /// Resolve force flag: when --force, grace_period becomes 0 and cascade becomes Background.
    pub fn resolve(&mut self) {
        if self.force {
            self.grace_period = Some(0);
            self.cascade = CascadeStrategy::Background;
        }
    }

    /// Build query parameters for the DELETE request.
    pub fn query_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        if let Some(gp) = self.grace_period {
            params.push(("gracePeriodSeconds".to_string(), gp.to_string()));
        }

        if self.dry_run {
            params.push(("dryRun".to_string(), "All".to_string()));
        }

        params
    }

    /// Build the JSON body containing propagationPolicy for the DELETE request.
    pub fn delete_body(&self) -> Option<serde_json::Value> {
        let policy = self.cascade.propagation_policy();
        let mut body = serde_json::json!({
            "kind": "DeleteOptions",
            "apiVersion": "v1",
            "propagationPolicy": policy,
        });

        if let Some(gp) = self.grace_period {
            body["gracePeriodSeconds"] = serde_json::json!(gp);
        }

        Some(body)
    }

    /// Format the output message for a deleted resource.
    pub fn format_output(&self, resource_type: &str, name: &str) -> String {
        if self.output.as_deref() == Some("name") {
            format!("{}/{}", resource_type_to_kind(resource_type), name)
        } else {
            let operation = if self.force {
                "force deleted"
            } else {
                "deleted"
            };
            let dry_run_suffix = if self.dry_run { " (server dry run)" } else { "" };
            format!(
                "{} \"{}\" {}{}",
                resource_type_to_kind(resource_type),
                name,
                operation,
                dry_run_suffix,
            )
        }
    }
}

/// Map CLI resource type aliases to canonical kind strings for output.
fn resource_type_to_kind(resource_type: &str) -> &str {
    match resource_type {
        "pod" | "pods" => "pod",
        "service" | "services" | "svc" => "service",
        "deployment" | "deployments" | "deploy" => "deployment.apps",
        "statefulset" | "statefulsets" | "sts" => "statefulset.apps",
        "daemonset" | "daemonsets" | "ds" => "daemonset.apps",
        "replicaset" | "replicasets" | "rs" => "replicaset.apps",
        "job" | "jobs" => "job.batch",
        "cronjob" | "cronjobs" | "cj" => "cronjob.batch",
        "configmap" | "configmaps" | "cm" => "configmap",
        "secret" | "secrets" => "secret",
        "serviceaccount" | "serviceaccounts" | "sa" => "serviceaccount",
        "ingress" | "ingresses" | "ing" => "ingress.networking.k8s.io",
        "persistentvolumeclaim" | "persistentvolumeclaims" | "pvc" => "persistentvolumeclaim",
        "persistentvolume" | "persistentvolumes" | "pv" => "persistentvolume",
        "storageclass" | "storageclasses" | "sc" => "storageclass.storage.k8s.io",
        "namespace" | "namespaces" | "ns" => "namespace",
        "node" | "nodes" => "node",
        "role" | "roles" => "role.rbac.authorization.k8s.io",
        "rolebinding" | "rolebindings" => "rolebinding.rbac.authorization.k8s.io",
        "clusterrole" | "clusterroles" => "clusterrole.rbac.authorization.k8s.io",
        "clusterrolebinding" | "clusterrolebindings" => {
            "clusterrolebinding.rbac.authorization.k8s.io"
        }
        other => other,
    }
}

pub async fn execute_from_file(client: &ApiClient, file: &str) -> Result<()> {
    let contents = if file == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        buffer
    } else {
        fs::read_to_string(file).context("Failed to read file")?
    };

    // Support for multi-document YAML files
    let mut deleted_count = 0;
    for document in serde_yaml::Deserializer::from_str(&contents) {
        let value = serde_yaml::Value::deserialize(document)?;

        if value.is_null() {
            continue;
        }

        delete_resource(client, &value).await?;
        deleted_count += 1;
    }

    println!("Deleted {} resource(s) from file", deleted_count);
    Ok(())
}

async fn delete_resource(client: &ApiClient, value: &serde_yaml::Value) -> Result<()> {
    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .context("Missing 'kind' field")?;

    let metadata = value.get("metadata").context("Missing 'metadata' field")?;
    let name = metadata
        .get("name")
        .and_then(|n| n.as_str())
        .context("Missing 'name' in metadata")?;

    let namespace = metadata.get("namespace").and_then(|n| n.as_str());

    // Construct API path based on resource kind
    let api_path = get_delete_api_path(kind, namespace, name)?;

    client
        .delete(&api_path)
        .await
        .context(format!("Failed to delete {} {}", kind, name))?;

    println!("{}/{} deleted", kind.to_lowercase(), name);
    Ok(())
}

fn get_delete_api_path(kind: &str, namespace: Option<&str>, name: &str) -> Result<String> {
    let ns = namespace.unwrap_or("default");
    Ok(match kind {
        "Pod" => format!("/api/v1/namespaces/{}/pods/{}", ns, name),
        "Service" => format!("/api/v1/namespaces/{}/services/{}", ns, name),
        "Deployment" => format!("/apis/apps/v1/namespaces/{}/deployments/{}", ns, name),
        "StatefulSet" => format!("/apis/apps/v1/namespaces/{}/statefulsets/{}", ns, name),
        "DaemonSet" => format!("/apis/apps/v1/namespaces/{}/daemonsets/{}", ns, name),
        "ReplicaSet" => format!("/apis/apps/v1/namespaces/{}/replicasets/{}", ns, name),
        "Job" => format!("/apis/batch/v1/namespaces/{}/jobs/{}", ns, name),
        "CronJob" => format!("/apis/batch/v1/namespaces/{}/cronjobs/{}", ns, name),
        "ConfigMap" => format!("/api/v1/namespaces/{}/configmaps/{}", ns, name),
        "Secret" => format!("/api/v1/namespaces/{}/secrets/{}", ns, name),
        "ServiceAccount" => format!("/api/v1/namespaces/{}/serviceaccounts/{}", ns, name),
        "Ingress" => format!(
            "/apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}",
            ns, name
        ),
        "PersistentVolumeClaim" => {
            format!("/api/v1/namespaces/{}/persistentvolumeclaims/{}", ns, name)
        }
        "PersistentVolume" => format!("/api/v1/persistentvolumes/{}", name),
        "StorageClass" => format!("/apis/storage.k8s.io/v1/storageclasses/{}", name),
        "Namespace" => format!("/api/v1/namespaces/{}", name),
        "Node" => format!("/api/v1/nodes/{}", name),
        "Role" => format!(
            "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}",
            ns, name
        ),
        "RoleBinding" => format!(
            "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}",
            ns, name
        ),
        "ClusterRole" => format!("/apis/rbac.authorization.k8s.io/v1/clusterroles/{}", name),
        "ClusterRoleBinding" => format!(
            "/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}",
            name
        ),
        _ => anyhow::bail!("Unsupported resource kind for deletion: {}", kind),
    })
}

pub async fn execute_with_selector(
    client: &ApiClient,
    resource_type: &str,
    selector: &str,
    namespace: &str,
    opts: &DeleteOptions,
) -> Result<()> {
    // Build the list API path with label selector
    let api_path = get_list_api_path(resource_type, namespace)?;
    let selector_query = format!("?labelSelector={}", urlencoding::encode(selector));
    let full_path = format!("{}{}", api_path, selector_query);

    // Fetch resources matching the selector
    let response: serde_json::Value = client
        .get(&full_path)
        .await
        .context("Failed to list resources with selector")?;

    let items = response
        .get("items")
        .and_then(|i| i.as_array())
        .context("No items in response")?;

    if items.is_empty() {
        println!("No resources found matching selector: {}", selector);
        return Ok(());
    }

    // Delete each resource
    for item in items {
        let name = item
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .context("Missing resource name")?;

        delete_single_resource(client, resource_type, name, Some(namespace), opts).await?;
    }

    Ok(())
}

/// Execute --all: delete all resources of the given type in the namespace.
pub async fn execute_delete_all(
    client: &ApiClient,
    resource_type: &str,
    namespace: &str,
    opts: &DeleteOptions,
) -> Result<()> {
    let collection_path = get_list_api_path(resource_type, namespace)?;

    // List all resources first so we can print their names
    let response: serde_json::Value = client
        .get(&collection_path)
        .await
        .context("Failed to list resources")?;

    let items = response
        .get("items")
        .and_then(|i| i.as_array())
        .context("No items in response")?;

    if items.is_empty() {
        println!("No resources found");
        return Ok(());
    }

    // Delete each individually (to get per-resource output and wait behavior)
    for item in items {
        let name = item
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .context("Missing resource name")?;

        delete_single_resource(client, resource_type, name, Some(namespace), opts).await?;
    }

    Ok(())
}

fn get_list_api_path(resource_type: &str, namespace: &str) -> Result<String> {
    Ok(match resource_type {
        "pod" | "pods" => format!("/api/v1/namespaces/{}/pods", namespace),
        "service" | "services" | "svc" => format!("/api/v1/namespaces/{}/services", namespace),
        "deployment" | "deployments" | "deploy" => {
            format!("/apis/apps/v1/namespaces/{}/deployments", namespace)
        }
        "statefulset" | "statefulsets" | "sts" => {
            format!("/apis/apps/v1/namespaces/{}/statefulsets", namespace)
        }
        "daemonset" | "daemonsets" | "ds" => {
            format!("/apis/apps/v1/namespaces/{}/daemonsets", namespace)
        }
        "replicaset" | "replicasets" | "rs" => {
            format!("/apis/apps/v1/namespaces/{}/replicasets", namespace)
        }
        "job" | "jobs" => format!("/apis/batch/v1/namespaces/{}/jobs", namespace),
        "cronjob" | "cronjobs" | "cj" => {
            format!("/apis/batch/v1/namespaces/{}/cronjobs", namespace)
        }
        "configmap" | "configmaps" | "cm" => {
            format!("/api/v1/namespaces/{}/configmaps", namespace)
        }
        "secret" | "secrets" => format!("/api/v1/namespaces/{}/secrets", namespace),
        "serviceaccount" | "serviceaccounts" | "sa" => {
            format!("/api/v1/namespaces/{}/serviceaccounts", namespace)
        }
        "node" | "nodes" => "/api/v1/nodes".to_string(),
        "namespace" | "namespaces" | "ns" => "/api/v1/namespaces".to_string(),
        "persistentvolume" | "persistentvolumes" | "pv" => {
            "/api/v1/persistentvolumes".to_string()
        }
        "clusterrole" | "clusterroles" => {
            "/apis/rbac.authorization.k8s.io/v1/clusterroles".to_string()
        }
        "clusterrolebinding" | "clusterrolebindings" => {
            "/apis/rbac.authorization.k8s.io/v1/clusterrolebindings".to_string()
        }
        "role" | "roles" => {
            format!(
                "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles",
                namespace
            )
        }
        "rolebinding" | "rolebindings" => {
            format!(
                "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings",
                namespace
            )
        }
        _ => anyhow::bail!(
            "Unsupported resource type for deletion: {}",
            resource_type
        ),
    })
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

/// Get the API path for a single named resource, given CLI resource type aliases.
fn get_resource_api_path(resource_type: &str, name: &str, namespace: &str) -> Result<String> {
    Ok(match resource_type {
        "pod" | "pods" => format!("/api/v1/namespaces/{}/pods/{}", namespace, name),
        "service" | "services" | "svc" => {
            format!("/api/v1/namespaces/{}/services/{}", namespace, name)
        }
        "deployment" | "deployments" | "deploy" => {
            format!(
                "/apis/apps/v1/namespaces/{}/deployments/{}",
                namespace, name
            )
        }
        "statefulset" | "statefulsets" | "sts" => {
            format!(
                "/apis/apps/v1/namespaces/{}/statefulsets/{}",
                namespace, name
            )
        }
        "daemonset" | "daemonsets" | "ds" => {
            format!(
                "/apis/apps/v1/namespaces/{}/daemonsets/{}",
                namespace, name
            )
        }
        "replicaset" | "replicasets" | "rs" => {
            format!(
                "/apis/apps/v1/namespaces/{}/replicasets/{}",
                namespace, name
            )
        }
        "job" | "jobs" => format!("/apis/batch/v1/namespaces/{}/jobs/{}", namespace, name),
        "cronjob" | "cronjobs" | "cj" => {
            format!("/apis/batch/v1/namespaces/{}/cronjobs/{}", namespace, name)
        }
        "configmap" | "configmaps" | "cm" => {
            format!("/api/v1/namespaces/{}/configmaps/{}", namespace, name)
        }
        "secret" | "secrets" => {
            format!("/api/v1/namespaces/{}/secrets/{}", namespace, name)
        }
        "serviceaccount" | "serviceaccounts" | "sa" => {
            format!(
                "/api/v1/namespaces/{}/serviceaccounts/{}",
                namespace, name
            )
        }
        "ingress" | "ingresses" | "ing" => {
            format!(
                "/apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}",
                namespace, name
            )
        }
        "persistentvolumeclaim" | "persistentvolumeclaims" | "pvc" => {
            format!(
                "/api/v1/namespaces/{}/persistentvolumeclaims/{}",
                namespace, name
            )
        }
        "persistentvolume" | "persistentvolumes" | "pv" => {
            format!("/api/v1/persistentvolumes/{}", name)
        }
        "storageclass" | "storageclasses" | "sc" => {
            format!("/apis/storage.k8s.io/v1/storageclasses/{}", name)
        }
        "node" | "nodes" => format!("/api/v1/nodes/{}", name),
        "namespace" | "namespaces" | "ns" => format!("/api/v1/namespaces/{}", name),
        "role" | "roles" => {
            format!(
                "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}",
                namespace, name
            )
        }
        "rolebinding" | "rolebindings" => {
            format!(
                "/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}",
                namespace, name
            )
        }
        "clusterrole" | "clusterroles" => {
            format!("/apis/rbac.authorization.k8s.io/v1/clusterroles/{}", name)
        }
        "clusterrolebinding" | "clusterrolebindings" => {
            format!(
                "/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}",
                name
            )
        }
        _ => anyhow::bail!("Unknown resource type: {}", resource_type),
    })
}

/// Core single-resource delete with full options support.
async fn delete_single_resource(
    client: &ApiClient,
    resource_type: &str,
    name: &str,
    namespace: Option<&str>,
    opts: &DeleteOptions,
) -> Result<()> {
    let ns = namespace.unwrap_or("default");
    let api_path = get_resource_api_path(resource_type, name, ns)?;

    let query_params = opts.query_params();
    let body = opts.delete_body();

    let status = client
        .delete_with_options(&api_path, &query_params, body.as_ref())
        .await
        .context(format!("Failed to delete {} {}", resource_type, name))?;

    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("{} \"{}\" not found", resource_type_to_kind(resource_type), name);
    }

    println!("{}", opts.format_output(resource_type, name));

    // --wait: poll until the resource is gone
    if opts.wait && !opts.dry_run {
        wait_for_deletion(client, &api_path).await?;
    }

    Ok(())
}

/// Poll until the resource returns 404 (deleted).
async fn wait_for_deletion(client: &ApiClient, api_path: &str) -> Result<()> {
    let timeout = std::time::Duration::from_secs(60);
    let poll_interval = std::time::Duration::from_millis(500);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Timed out waiting for resource to be deleted");
        }
        let exists = client
            .resource_exists(api_path)
            .await
            .context("Failed polling resource for deletion")?;
        if !exists {
            return Ok(());
        }
        tokio::time::sleep(poll_interval).await;
    }
}

pub async fn execute_enhanced(
    client: &ApiClient,
    resource_type: &str,
    name: &str,
    namespace: &str,
    opts: &DeleteOptions,
) -> Result<()> {
    delete_single_resource(client, resource_type, name, Some(namespace), opts).await
}

pub async fn execute(
    client: &ApiClient,
    resource_type: &str,
    name: &str,
    namespace: Option<&str>,
) -> Result<()> {
    let opts = DeleteOptions::default();
    delete_single_resource(client, resource_type, name, namespace, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_delete_api_path_pod() {
        let path = get_delete_api_path("Pod", Some("default"), "my-pod").unwrap();
        assert_eq!(path, "/api/v1/namespaces/default/pods/my-pod");
    }

    #[test]
    fn test_get_delete_api_path_deployment() {
        let path = get_delete_api_path("Deployment", Some("prod"), "my-deploy").unwrap();
        assert_eq!(
            path,
            "/apis/apps/v1/namespaces/prod/deployments/my-deploy"
        );
    }

    #[test]
    fn test_get_delete_api_path_namespace_is_cluster_scoped() {
        let path = get_delete_api_path("Namespace", None, "kube-system").unwrap();
        assert_eq!(path, "/api/v1/namespaces/kube-system");
    }

    #[test]
    fn test_get_delete_api_path_node_is_cluster_scoped() {
        let path = get_delete_api_path("Node", None, "node-1").unwrap();
        assert_eq!(path, "/api/v1/nodes/node-1");
    }

    #[test]
    fn test_get_delete_api_path_clusterrole() {
        let path = get_delete_api_path("ClusterRole", None, "admin").unwrap();
        assert_eq!(
            path,
            "/apis/rbac.authorization.k8s.io/v1/clusterroles/admin"
        );
    }

    #[test]
    fn test_get_delete_api_path_unsupported() {
        let result = get_delete_api_path("UnknownKind", None, "foo");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_delete_api_path_default_namespace() {
        let path = get_delete_api_path("Pod", None, "my-pod").unwrap();
        assert_eq!(path, "/api/v1/namespaces/default/pods/my-pod");
    }

    #[test]
    fn test_get_list_api_path_pods() {
        let path = get_list_api_path("pods", "default").unwrap();
        assert_eq!(path, "/api/v1/namespaces/default/pods");
    }

    #[test]
    fn test_get_list_api_path_deploy_alias() {
        let path = get_list_api_path("deploy", "staging").unwrap();
        assert_eq!(path, "/apis/apps/v1/namespaces/staging/deployments");
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding::encode("app=web"), "app=web");
        assert_eq!(urlencoding::encode("a b"), "a+b");
    }

    // === New tests for the 7 fixed issues ===

    #[test]
    fn test_grace_period_query_param() {
        let opts = DeleteOptions {
            grace_period: Some(30),
            ..Default::default()
        };
        let params = opts.query_params();
        assert!(params.contains(&("gracePeriodSeconds".to_string(), "30".to_string())));
    }

    #[test]
    fn test_grace_period_zero_query_param() {
        let opts = DeleteOptions {
            grace_period: Some(0),
            ..Default::default()
        };
        let params = opts.query_params();
        assert!(params.contains(&("gracePeriodSeconds".to_string(), "0".to_string())));
    }

    #[test]
    fn test_force_sets_grace_period_zero_and_background() {
        let mut opts = DeleteOptions {
            force: true,
            ..Default::default()
        };
        opts.resolve();
        assert_eq!(opts.grace_period, Some(0));
        assert_eq!(opts.cascade, CascadeStrategy::Background);

        let params = opts.query_params();
        assert!(params.contains(&("gracePeriodSeconds".to_string(), "0".to_string())));

        let body = opts.delete_body().unwrap();
        assert_eq!(body["propagationPolicy"], "Background");
        assert_eq!(body["gracePeriodSeconds"], 0);
    }

    #[test]
    fn test_cascade_foreground_body() {
        let opts = DeleteOptions {
            cascade: CascadeStrategy::Foreground,
            ..Default::default()
        };
        let body = opts.delete_body().unwrap();
        assert_eq!(body["propagationPolicy"], "Foreground");
    }

    #[test]
    fn test_cascade_orphan_body() {
        let opts = DeleteOptions {
            cascade: CascadeStrategy::Orphan,
            ..Default::default()
        };
        let body = opts.delete_body().unwrap();
        assert_eq!(body["propagationPolicy"], "Orphan");
    }

    #[test]
    fn test_cascade_background_body() {
        let opts = DeleteOptions {
            cascade: CascadeStrategy::Background,
            ..Default::default()
        };
        let body = opts.delete_body().unwrap();
        assert_eq!(body["propagationPolicy"], "Background");
    }

    #[test]
    fn test_cascade_from_str() {
        assert_eq!(
            CascadeStrategy::from_str_value("foreground").unwrap(),
            CascadeStrategy::Foreground
        );
        assert_eq!(
            CascadeStrategy::from_str_value("background").unwrap(),
            CascadeStrategy::Background
        );
        assert_eq!(
            CascadeStrategy::from_str_value("orphan").unwrap(),
            CascadeStrategy::Orphan
        );
        assert!(CascadeStrategy::from_str_value("invalid").is_err());
    }

    #[test]
    fn test_dry_run_query_param() {
        let opts = DeleteOptions {
            dry_run: true,
            ..Default::default()
        };
        let params = opts.query_params();
        assert!(params.contains(&("dryRun".to_string(), "All".to_string())));
    }

    #[test]
    fn test_dry_run_not_set_by_default() {
        let opts = DeleteOptions::default();
        let params = opts.query_params();
        assert!(!params.iter().any(|(k, _)| k == "dryRun"));
    }

    #[test]
    fn test_output_name_format() {
        let opts = DeleteOptions {
            output: Some("name".to_string()),
            ..Default::default()
        };
        let output = opts.format_output("pod", "nginx");
        assert_eq!(output, "pod/nginx");
    }

    #[test]
    fn test_output_name_format_deployment() {
        let opts = DeleteOptions {
            output: Some("name".to_string()),
            ..Default::default()
        };
        let output = opts.format_output("deployment", "web");
        assert_eq!(output, "deployment.apps/web");
    }

    #[test]
    fn test_output_default_format() {
        let opts = DeleteOptions::default();
        let output = opts.format_output("pod", "nginx");
        assert_eq!(output, "pod \"nginx\" deleted");
    }

    #[test]
    fn test_output_force_format() {
        let mut opts = DeleteOptions {
            force: true,
            ..Default::default()
        };
        opts.resolve();
        let output = opts.format_output("pod", "nginx");
        assert_eq!(output, "pod \"nginx\" force deleted");
    }

    #[test]
    fn test_output_dry_run_format() {
        let opts = DeleteOptions {
            dry_run: true,
            ..Default::default()
        };
        let output = opts.format_output("pod", "nginx");
        assert_eq!(output, "pod \"nginx\" deleted (server dry run)");
    }

    #[test]
    fn test_combined_force_and_dry_run_query_params() {
        let mut opts = DeleteOptions {
            force: true,
            dry_run: true,
            ..Default::default()
        };
        opts.resolve();
        let params = opts.query_params();
        assert!(params.contains(&("gracePeriodSeconds".to_string(), "0".to_string())));
        assert!(params.contains(&("dryRun".to_string(), "All".to_string())));
    }

    #[test]
    fn test_get_resource_api_path_pod() {
        let path = get_resource_api_path("pod", "nginx", "default").unwrap();
        assert_eq!(path, "/api/v1/namespaces/default/pods/nginx");
    }

    #[test]
    fn test_get_resource_api_path_deploy_alias() {
        let path = get_resource_api_path("deploy", "web", "prod").unwrap();
        assert_eq!(path, "/apis/apps/v1/namespaces/prod/deployments/web");
    }

    #[test]
    fn test_get_resource_api_path_cluster_scoped_node() {
        let path = get_resource_api_path("node", "node-1", "default").unwrap();
        assert_eq!(path, "/api/v1/nodes/node-1");
    }

    #[test]
    fn test_get_resource_api_path_cluster_scoped_namespace() {
        let path = get_resource_api_path("namespace", "kube-system", "default").unwrap();
        assert_eq!(path, "/api/v1/namespaces/kube-system");
    }

    #[test]
    fn test_get_resource_api_path_unsupported() {
        let result = get_resource_api_path("unknown", "foo", "default");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_body_includes_kind_and_api_version() {
        let opts = DeleteOptions::default();
        let body = opts.delete_body().unwrap();
        assert_eq!(body["kind"], "DeleteOptions");
        assert_eq!(body["apiVersion"], "v1");
    }

    #[test]
    fn test_delete_body_grace_period_in_body() {
        let opts = DeleteOptions {
            grace_period: Some(60),
            ..Default::default()
        };
        let body = opts.delete_body().unwrap();
        assert_eq!(body["gracePeriodSeconds"], 60);
    }

    #[test]
    fn test_delete_body_no_grace_period_when_none() {
        let opts = DeleteOptions::default();
        let body = opts.delete_body().unwrap();
        assert!(body.get("gracePeriodSeconds").is_none());
    }

    #[test]
    fn test_list_api_path_cluster_scoped_nodes() {
        let path = get_list_api_path("nodes", "default").unwrap();
        assert_eq!(path, "/api/v1/nodes");
    }

    #[test]
    fn test_list_api_path_cluster_scoped_namespaces() {
        let path = get_list_api_path("namespaces", "default").unwrap();
        assert_eq!(path, "/api/v1/namespaces");
    }

    #[test]
    fn test_resource_type_to_kind_core() {
        assert_eq!(resource_type_to_kind("pod"), "pod");
        assert_eq!(resource_type_to_kind("pods"), "pod");
        assert_eq!(resource_type_to_kind("service"), "service");
        assert_eq!(resource_type_to_kind("svc"), "service");
    }

    #[test]
    fn test_resource_type_to_kind_with_group() {
        assert_eq!(resource_type_to_kind("deployment"), "deployment.apps");
        assert_eq!(resource_type_to_kind("deploy"), "deployment.apps");
        assert_eq!(resource_type_to_kind("job"), "job.batch");
        assert_eq!(resource_type_to_kind("clusterrole"), "clusterrole.rbac.authorization.k8s.io");
    }
}

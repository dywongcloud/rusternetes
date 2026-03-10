use crate::client::ApiClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::io::{self, Read};

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

    let namespace = metadata
        .get("namespace")
        .and_then(|n| n.as_str());

    // Construct API path based on resource kind
    let api_path = get_delete_api_path(kind, namespace, name)?;

    client.delete(&api_path).await
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
        "Ingress" => format!("/apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}", ns, name),
        "PersistentVolumeClaim" => format!("/api/v1/namespaces/{}/persistentvolumeclaims/{}", ns, name),
        "PersistentVolume" => format!("/api/v1/persistentvolumes/{}", name),
        "StorageClass" => format!("/apis/storage.k8s.io/v1/storageclasses/{}", name),
        "Namespace" => format!("/api/v1/namespaces/{}", name),
        "Node" => format!("/api/v1/nodes/{}", name),
        "Role" => format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}", ns, name),
        "RoleBinding" => format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}", ns, name),
        "ClusterRole" => format!("/apis/rbac.authorization.k8s.io/v1/clusterroles/{}", name),
        "ClusterRoleBinding" => format!("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}", name),
        _ => anyhow::bail!("Unsupported resource kind for deletion: {}", kind),
    })
}

pub async fn execute_with_selector(client: &ApiClient, resource_type: &str, selector: &str, namespace: &str) -> Result<()> {
    // Build the list API path with label selector
    let api_path = get_list_api_path(resource_type, namespace)?;
    let selector_query = format!("?labelSelector={}", urlencoding::encode(selector));
    let full_path = format!("{}{}", api_path, selector_query);

    // Fetch resources matching the selector
    let response: serde_json::Value = client.get(&full_path).await
        .context("Failed to list resources with selector")?;

    let items = response.get("items")
        .and_then(|i| i.as_array())
        .context("No items in response")?;

    if items.is_empty() {
        println!("No resources found matching selector: {}", selector);
        return Ok(());
    }

    println!("Found {} resource(s) matching selector {}", items.len(), selector);

    // Delete each resource
    let mut deleted_count = 0;
    for item in items {
        let name = item.get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .context("Missing resource name")?;

        execute(client, resource_type, name, Some(namespace)).await?;
        deleted_count += 1;
    }

    println!("Deleted {} resource(s)", deleted_count);
    Ok(())
}

fn get_list_api_path(resource_type: &str, namespace: &str) -> Result<String> {
    Ok(match resource_type {
        "pod" | "pods" => format!("/api/v1/namespaces/{}/pods", namespace),
        "service" | "services" | "svc" => format!("/api/v1/namespaces/{}/services", namespace),
        "deployment" | "deployments" | "deploy" => format!("/apis/apps/v1/namespaces/{}/deployments", namespace),
        "statefulset" | "statefulsets" | "sts" => format!("/apis/apps/v1/namespaces/{}/statefulsets", namespace),
        "daemonset" | "daemonsets" | "ds" => format!("/apis/apps/v1/namespaces/{}/daemonsets", namespace),
        "replicaset" | "replicasets" | "rs" => format!("/apis/apps/v1/namespaces/{}/replicasets", namespace),
        "job" | "jobs" => format!("/apis/batch/v1/namespaces/{}/jobs", namespace),
        "cronjob" | "cronjobs" | "cj" => format!("/apis/batch/v1/namespaces/{}/cronjobs", namespace),
        "configmap" | "configmaps" | "cm" => format!("/api/v1/namespaces/{}/configmaps", namespace),
        "secret" | "secrets" => format!("/api/v1/namespaces/{}/secrets", namespace),
        _ => anyhow::bail!("Unsupported resource type for selector deletion: {}", resource_type),
    })
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' | '=' | ',' | '!' => c.to_string(),
                ' ' => "+".to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
}

pub async fn execute_enhanced(
    client: &ApiClient,
    resource_type: &str,
    name: &str,
    namespace: &str,
    force: bool,
    grace_period: Option<i64>,
) -> Result<()> {
    if force {
        println!("Force deletion enabled");
    }
    if let Some(gp) = grace_period {
        println!("Grace period: {} seconds", gp);
    }
    execute(client, resource_type, name, Some(namespace)).await
}

pub async fn execute(
    client: &ApiClient,
    resource_type: &str,
    name: &str,
    namespace: Option<&str>,
) -> Result<()> {
    let default_namespace = "default";
    let ns = namespace.unwrap_or(default_namespace);

    match resource_type {
        "pod" | "pods" => {
            client
                .delete(&format!("/api/v1/namespaces/{}/pods/{}", ns, name))
                .await?;
            println!("Pod '{}' deleted", name);
        }
        "service" | "services" | "svc" => {
            client
                .delete(&format!("/api/v1/namespaces/{}/services/{}", ns, name))
                .await?;
            println!("Service '{}' deleted", name);
        }
        "deployment" | "deployments" | "deploy" => {
            client
                .delete(&format!(
                    "/apis/apps/v1/namespaces/{}/deployments/{}",
                    ns, name
                ))
                .await?;
            println!("Deployment '{}' deleted", name);
        }
        "node" | "nodes" => {
            client.delete(&format!("/api/v1/nodes/{}", name)).await?;
            println!("Node '{}' deleted", name);
        }
        "namespace" | "namespaces" | "ns" => {
            client
                .delete(&format!("/api/v1/namespaces/{}", name))
                .await?;
            println!("Namespace '{}' deleted", name);
        }
        "clusterrole" | "clusterroles" => {
            client
                .delete(&format!("/apis/rbac.authorization.k8s.io/v1/clusterroles/{}", name))
                .await?;
            println!("ClusterRole '{}' deleted", name);
        }
        "clusterrolebinding" | "clusterrolebindings" => {
            client
                .delete(&format!("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}", name))
                .await?;
            println!("ClusterRoleBinding '{}' deleted", name);
        }
        "role" | "roles" => {
            client
                .delete(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}", ns, name))
                .await?;
            println!("Role '{}' deleted", name);
        }
        "rolebinding" | "rolebindings" => {
            client
                .delete(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}", ns, name))
                .await?;
            println!("RoleBinding '{}' deleted", name);
        }
        _ => anyhow::bail!("Unknown resource type: {}", resource_type),
    }

    Ok(())
}

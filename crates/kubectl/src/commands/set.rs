use crate::client::ApiClient;
use crate::types::SetCommands;
use anyhow::{Context, Result};
use serde_json::{json, Value};

/// Dispatch set subcommands.
pub async fn execute_set(
    client: &ApiClient,
    command: SetCommands,
    default_namespace: &str,
) -> Result<()> {
    match command {
        SetCommands::Image {
            resource,
            container_images,
            namespace,
        } => {
            execute_image(
                client,
                &resource,
                &container_images,
                namespace.as_deref().unwrap_or(default_namespace),
            )
            .await
        }
        SetCommands::Env {
            resource,
            env_vars,
            namespace,
            container,
            list,
        } => {
            execute_env(
                client,
                &resource,
                &env_vars,
                namespace.as_deref().unwrap_or(default_namespace),
                container.as_deref(),
                list,
            )
            .await
        }
        SetCommands::Resources {
            resource,
            namespace,
            container,
            limits,
            requests,
        } => {
            execute_resources(
                client,
                &resource,
                namespace.as_deref().unwrap_or(default_namespace),
                container.as_deref(),
                limits.as_deref(),
                requests.as_deref(),
            )
            .await
        }
    }
}

/// Resolve the API path for a resource type/name.
fn resolve_resource_path(
    resource_type: &str,
    name: &str,
    namespace: &str,
) -> Result<(String, String)> {
    let (api_path, resource_name) = match resource_type {
        "pod" | "pods" | "po" => ("api/v1", "pods"),
        "deployment" | "deployments" | "deploy" => ("apis/apps/v1", "deployments"),
        "daemonset" | "daemonsets" | "ds" => ("apis/apps/v1", "daemonsets"),
        "statefulset" | "statefulsets" | "sts" => ("apis/apps/v1", "statefulsets"),
        "replicaset" | "replicasets" | "rs" => ("apis/apps/v1", "replicasets"),
        "replicationcontroller" | "replicationcontrollers" | "rc" => {
            ("api/v1", "replicationcontrollers")
        }
        "cronjob" | "cronjobs" | "cj" => ("apis/batch/v1", "cronjobs"),
        "job" | "jobs" => ("apis/batch/v1", "jobs"),
        _ => anyhow::bail!("Unsupported resource type for set: {}", resource_type),
    };

    let path = format!(
        "/{}/namespaces/{}/{}/{}",
        api_path, namespace, resource_name, name
    );

    Ok((path, resource_name.to_string()))
}

/// Parse "resource_type/name" format into (type, name).
fn parse_resource_arg(arg: &str) -> Result<(String, String)> {
    if let Some((rt, name)) = arg.split_once('/') {
        Ok((rt.to_string(), name.to_string()))
    } else {
        anyhow::bail!(
            "Invalid resource format: '{}'. Expected TYPE/NAME (e.g., deployment/nginx)",
            arg
        );
    }
}

/// Update container images in a resource.
///
/// Usage: `kubectl set image deployment/nginx nginx=nginx:1.9.1`
pub async fn execute_image(
    client: &ApiClient,
    resource: &str,
    container_images: &[String],
    namespace: &str,
) -> Result<()> {
    let (resource_type, name) = parse_resource_arg(resource)?;
    let (path, _) = resolve_resource_path(&resource_type, &name, namespace)?;

    // Parse container=image pairs
    let mut image_updates: Vec<(String, String)> = Vec::new();
    for ci in container_images {
        if let Some((container, image)) = ci.split_once('=') {
            image_updates.push((container.to_string(), image.to_string()));
        } else {
            anyhow::bail!(
                "Invalid image format: '{}'. Expected CONTAINER=IMAGE",
                ci
            );
        }
    }

    if image_updates.is_empty() {
        anyhow::bail!("At least one CONTAINER=IMAGE argument is required");
    }

    // Get the current resource to find container indices
    let current: Value = client
        .get(&path)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Failed to get resource")?;

    // Find the containers array path (handles both pod and pod template specs)
    let containers = if current.get("spec").and_then(|s| s.get("template")).is_some() {
        // Deployment/StatefulSet/DaemonSet/ReplicaSet/Job/CronJob
        current["spec"]["template"]["spec"]["containers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        // Pod
        current["spec"]["containers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };

    // Build the strategic merge patch for containers
    let mut patch_containers: Vec<Value> = Vec::new();
    for (container_name, image) in &image_updates {
        if container_name == "*" {
            // Update all containers
            for c in &containers {
                if let Some(cname) = c.get("name").and_then(|n| n.as_str()) {
                    patch_containers.push(json!({
                        "name": cname,
                        "image": image,
                    }));
                }
            }
        } else {
            // Check container exists
            let found = containers
                .iter()
                .any(|c| c.get("name").and_then(|n| n.as_str()) == Some(container_name));
            if !found {
                anyhow::bail!(
                    "Container '{}' not found in resource {}/{}",
                    container_name,
                    resource_type,
                    name
                );
            }
            patch_containers.push(json!({
                "name": container_name,
                "image": image,
            }));
        }
    }

    // Build patch based on resource type
    let patch_body = if current.get("spec").and_then(|s| s.get("template")).is_some() {
        json!({
            "spec": {
                "template": {
                    "spec": {
                        "containers": patch_containers,
                    }
                }
            }
        })
    } else {
        json!({
            "spec": {
                "containers": patch_containers,
            }
        })
    };

    let _result: Value = client
        .patch(
            &path,
            &patch_body,
            "application/strategic-merge-patch+json",
        )
        .await
        .context("Failed to update image")?;

    for (container_name, image) in &image_updates {
        if container_name == "*" {
            println!(
                "{}/{} image updated to {} (all containers)",
                resource_type, name, image
            );
        } else {
            println!(
                "{}/{} container {} image updated to {}",
                resource_type, name, container_name, image
            );
        }
    }

    Ok(())
}

/// Update environment variables on a resource.
///
/// Usage: `kubectl set env deployment/registry STORAGE_DIR=/local`
pub async fn execute_env(
    client: &ApiClient,
    resource: &str,
    env_vars: &[String],
    namespace: &str,
    container_name: Option<&str>,
    list: bool,
) -> Result<()> {
    let (resource_type, name) = parse_resource_arg(resource)?;
    let (path, _) = resolve_resource_path(&resource_type, &name, namespace)?;

    // Get current resource
    let current: Value = client
        .get(&path)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Failed to get resource")?;

    let containers = if current.get("spec").and_then(|s| s.get("template")).is_some() {
        current["spec"]["template"]["spec"]["containers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        current["spec"]["containers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };

    // If --list, just print current env vars
    if list {
        for c in &containers {
            let cname = c.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
            if let Some(target) = container_name {
                if cname != target {
                    continue;
                }
            }
            println!("# Container: {}", cname);
            if let Some(envs) = c.get("env").and_then(|e| e.as_array()) {
                for env in envs {
                    let key = env.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let val = env.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    println!("{}={}", key, val);
                }
            }
        }
        return Ok(());
    }

    // Parse env vars: KEY=VALUE to add, KEY- to remove
    let mut to_add: Vec<(String, String)> = Vec::new();
    let mut to_remove: Vec<String> = Vec::new();

    for ev in env_vars {
        if let Some(key) = ev.strip_suffix('-') {
            to_remove.push(key.to_string());
        } else if let Some((key, value)) = ev.split_once('=') {
            to_add.push((key.to_string(), value.to_string()));
        } else {
            anyhow::bail!(
                "Invalid env format: '{}'. Expected KEY=VALUE or KEY-",
                ev
            );
        }
    }

    // Build updated containers with env changes
    let mut updated_containers: Vec<Value> = Vec::new();

    for c in &containers {
        let cname = c.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if let Some(target) = container_name {
            if cname != target {
                continue;
            }
        }

        let mut env_list: Vec<Value> = c
            .get("env")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();

        // Remove specified vars
        env_list.retain(|e| {
            let key = e.get("name").and_then(|n| n.as_str()).unwrap_or("");
            !to_remove.contains(&key.to_string())
        });

        // Add/update vars
        for (key, value) in &to_add {
            if let Some(existing) = env_list
                .iter_mut()
                .find(|e| e.get("name").and_then(|n| n.as_str()) == Some(key))
            {
                existing["value"] = json!(value);
            } else {
                env_list.push(json!({"name": key, "value": value}));
            }
        }

        updated_containers.push(json!({
            "name": cname,
            "env": env_list,
        }));
    }

    let patch_body = if current.get("spec").and_then(|s| s.get("template")).is_some() {
        json!({
            "spec": {
                "template": {
                    "spec": {
                        "containers": updated_containers,
                    }
                }
            }
        })
    } else {
        json!({
            "spec": {
                "containers": updated_containers,
            }
        })
    };

    let _result: Value = client
        .patch(
            &path,
            &patch_body,
            "application/strategic-merge-patch+json",
        )
        .await
        .context("Failed to update environment variables")?;

    println!("{}/{} env updated", resource_type, name);

    Ok(())
}

/// Update resource requests/limits on a resource.
///
/// Usage: `kubectl set resources deployment nginx --limits=cpu=200m,memory=512Mi`
pub async fn execute_resources(
    client: &ApiClient,
    resource: &str,
    namespace: &str,
    container_name: Option<&str>,
    limits: Option<&str>,
    requests: Option<&str>,
) -> Result<()> {
    let (resource_type, name) = parse_resource_arg(resource)?;
    let (path, _) = resolve_resource_path(&resource_type, &name, namespace)?;

    // Get current resource
    let current: Value = client
        .get(&path)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Failed to get resource")?;

    let containers = if current.get("spec").and_then(|s| s.get("template")).is_some() {
        current["spec"]["template"]["spec"]["containers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        current["spec"]["containers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };

    // Parse limits and requests
    let limits_map = parse_resource_spec(limits)?;
    let requests_map = parse_resource_spec(requests)?;

    // Build updated containers
    let mut updated_containers: Vec<Value> = Vec::new();

    for c in &containers {
        let cname = c.get("name").and_then(|n| n.as_str()).unwrap_or("");

        if let Some(target) = container_name {
            if cname != target {
                continue;
            }
        }

        let mut resources = json!({});

        if !limits_map.is_empty() {
            resources["limits"] = json!(limits_map);
        }
        if !requests_map.is_empty() {
            resources["requests"] = json!(requests_map);
        }

        updated_containers.push(json!({
            "name": cname,
            "resources": resources,
        }));
    }

    let patch_body = if current.get("spec").and_then(|s| s.get("template")).is_some() {
        json!({
            "spec": {
                "template": {
                    "spec": {
                        "containers": updated_containers,
                    }
                }
            }
        })
    } else {
        json!({
            "spec": {
                "containers": updated_containers,
            }
        })
    };

    let _result: Value = client
        .patch(
            &path,
            &patch_body,
            "application/strategic-merge-patch+json",
        )
        .await
        .context("Failed to update resource requirements")?;

    println!("{}/{} resource requirements updated", resource_type, name);

    Ok(())
}

/// Parse a resource spec string like "cpu=200m,memory=512Mi" into a map.
fn parse_resource_spec(
    spec: Option<&str>,
) -> Result<std::collections::HashMap<String, String>> {
    let mut map = std::collections::HashMap::new();
    if let Some(s) = spec {
        for pair in s.split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            if let Some((key, value)) = pair.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            } else {
                anyhow::bail!("Invalid resource spec: '{}'. Expected key=value", pair);
            }
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resource_arg() {
        let (rt, name) = parse_resource_arg("deployment/nginx").unwrap();
        assert_eq!(rt, "deployment");
        assert_eq!(name, "nginx");
    }

    #[test]
    fn test_parse_resource_arg_invalid() {
        assert!(parse_resource_arg("nginx").is_err());
    }

    #[test]
    fn test_parse_resource_spec() {
        let map = parse_resource_spec(Some("cpu=200m,memory=512Mi")).unwrap();
        assert_eq!(map.get("cpu").unwrap(), "200m");
        assert_eq!(map.get("memory").unwrap(), "512Mi");
    }

    #[test]
    fn test_parse_resource_spec_none() {
        let map = parse_resource_spec(None).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_resolve_resource_path() {
        let (path, _) = resolve_resource_path("deployment", "nginx", "default").unwrap();
        assert_eq!(path, "/apis/apps/v1/namespaces/default/deployments/nginx");

        let (path, _) = resolve_resource_path("pod", "mypod", "kube-system").unwrap();
        assert_eq!(path, "/api/v1/namespaces/kube-system/pods/mypod");

        let (path, _) = resolve_resource_path("sts", "myapp", "default").unwrap();
        assert_eq!(
            path,
            "/apis/apps/v1/namespaces/default/statefulsets/myapp"
        );
    }

    #[test]
    fn test_resolve_unsupported() {
        assert!(resolve_resource_path("configmap", "test", "default").is_err());
    }
}

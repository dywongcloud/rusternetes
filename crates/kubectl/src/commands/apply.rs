use crate::client::ApiClient;
use anyhow::{Context, Result};
use rusternetes_common::resources::{Deployment, Namespace, Node, Pod, Service};
use std::fs;

pub async fn execute(client: &ApiClient, file: &str) -> Result<()> {
    let contents = fs::read_to_string(file).context("Failed to read file")?;
    let value: serde_yaml::Value = serde_yaml::from_str(&contents)?;

    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .context("Missing 'kind' field")?;

    match kind {
        "Pod" => {
            let pod: Pod = serde_yaml::from_str(&contents)?;
            let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Pod = client
                .put(
                    &format!(
                        "/api/v1/namespaces/{}/pods/{}",
                        namespace, pod.metadata.name
                    ),
                    &pod,
                )
                .await?;
            println!("Pod '{}' applied", pod.metadata.name);
        }
        "Service" => {
            let service: Service = serde_yaml::from_str(&contents)?;
            let namespace = service.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Service = client
                .put(
                    &format!(
                        "/api/v1/namespaces/{}/services/{}",
                        namespace, service.metadata.name
                    ),
                    &service,
                )
                .await?;
            println!("Service '{}' applied", service.metadata.name);
        }
        "Deployment" => {
            let deployment: Deployment = serde_yaml::from_str(&contents)?;
            let namespace = deployment.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Deployment = client
                .put(
                    &format!(
                        "/apis/apps/v1/namespaces/{}/deployments/{}",
                        namespace, deployment.metadata.name
                    ),
                    &deployment,
                )
                .await?;
            println!("Deployment '{}' applied", deployment.metadata.name);
        }
        "Node" => {
            let node: Node = serde_yaml::from_str(&contents)?;
            let _result: Node = client
                .put(&format!("/api/v1/nodes/{}", node.metadata.name), &node)
                .await?;
            println!("Node '{}' applied", node.metadata.name);
        }
        "Namespace" => {
            let namespace: Namespace = serde_yaml::from_str(&contents)?;
            let _result: Namespace = client
                .put(
                    &format!("/api/v1/namespaces/{}", namespace.metadata.name),
                    &namespace,
                )
                .await?;
            println!("Namespace '{}' applied", namespace.metadata.name);
        }
        _ => anyhow::bail!("Unsupported resource kind: {}", kind),
    }

    Ok(())
}

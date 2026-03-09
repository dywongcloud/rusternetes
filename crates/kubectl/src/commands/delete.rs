use crate::client::ApiClient;
use anyhow::Result;

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
        _ => anyhow::bail!("Unknown resource type: {}", resource_type),
    }

    Ok(())
}

use crate::client::ApiClient;
use anyhow::{Context, Result};
use rusternetes_common::resources::{Deployment, Namespace, Node, Pod, Service, Job, CronJob};
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
            let mut pod: Pod = serde_yaml::from_str(&contents)?;
            pod.metadata.ensure_uid();
            pod.metadata.ensure_creation_timestamp();
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
            let mut service: Service = serde_yaml::from_str(&contents)?;
            service.metadata.ensure_uid();
            service.metadata.ensure_creation_timestamp();
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
            let mut deployment: Deployment = serde_yaml::from_str(&contents)?;
            deployment.metadata.ensure_uid();
            deployment.metadata.ensure_creation_timestamp();
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
            let mut node: Node = serde_yaml::from_str(&contents)?;
            node.metadata.ensure_uid();
            node.metadata.ensure_creation_timestamp();
            let _result: Node = client
                .put(&format!("/api/v1/nodes/{}", node.metadata.name), &node)
                .await?;
            println!("Node '{}' applied", node.metadata.name);
        }
        "Namespace" => {
            let mut namespace: Namespace = serde_yaml::from_str(&contents)?;
            namespace.metadata.ensure_uid();
            namespace.metadata.ensure_creation_timestamp();
            let _result: Namespace = client
                .put(
                    &format!("/api/v1/namespaces/{}", namespace.metadata.name),
                    &namespace,
                )
                .await?;
            println!("Namespace '{}' applied", namespace.metadata.name);
        }
        "Job" => {
            let mut job: Job = serde_yaml::from_str(&contents)?;
            job.metadata.ensure_uid();
            job.metadata.ensure_creation_timestamp();
            let namespace = job.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Job = client
                .put(
                    &format!(
                        "/apis/batch/v1/namespaces/{}/jobs/{}",
                        namespace, job.metadata.name
                    ),
                    &job,
                )
                .await?;
            println!("Job '{}' applied", job.metadata.name);
        }
        "CronJob" => {
            let mut cronjob: CronJob = serde_yaml::from_str(&contents)?;
            cronjob.metadata.ensure_uid();
            cronjob.metadata.ensure_creation_timestamp();
            let namespace = cronjob.metadata.namespace.as_deref().unwrap_or("default");
            let _result: CronJob = client
                .put(
                    &format!(
                        "/apis/batch/v1/namespaces/{}/cronjobs/{}",
                        namespace, cronjob.metadata.name
                    ),
                    &cronjob,
                )
                .await?;
            println!("CronJob '{}' applied", cronjob.metadata.name);
        }
        _ => anyhow::bail!("Unsupported resource kind: {}", kind),
    }

    Ok(())
}

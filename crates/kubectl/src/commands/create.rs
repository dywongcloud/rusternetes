use crate::client::ApiClient;
use anyhow::{Context, Result};
use rusternetes_common::resources::{
    Deployment, Namespace, Node, Pod, Service, StorageClass,
    VolumeSnapshot, VolumeSnapshotClass, Endpoints, ResourceQuota, LimitRange, PriorityClass,
};
use serde::Deserialize;
use std::fs;

/// Execute inline resource creation (e.g., kubectl create namespace foo)
pub async fn execute_inline(client: &ApiClient, args: &[String], namespace: &str) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("Resource type required");
    }

    let resource_type = &args[0];
    match resource_type.as_str() {
        "namespace" | "ns" => {
            if args.len() < 2 {
                anyhow::bail!("Namespace name required");
            }
            let name = &args[1];
            println!("Creating namespace: {}", name);
            println!("Note: Inline resource creation not yet fully implemented");
        }
        _ => {
            println!("Creating {} in namespace {}", resource_type, namespace);
            println!("Note: Inline resource creation not yet fully implemented");
        }
    }

    Ok(())
}

pub async fn execute(client: &ApiClient, file: &str) -> Result<()> {
    let contents = fs::read_to_string(file).context("Failed to read file")?;

    // Support for multi-document YAML files
    for document in serde_yaml::Deserializer::from_str(&contents) {
        let value = serde_yaml::Value::deserialize(document)?;

        // Skip empty documents
        if value.is_null() {
            continue;
        }

        create_resource(client, &value).await?;
    }

    Ok(())
}

async fn create_resource(client: &ApiClient, value: &serde_yaml::Value) -> Result<()> {
    // Get the kind field
    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .context("Missing 'kind' field")?;

    let yaml_str = serde_yaml::to_string(value)?;

    match kind {
        "Pod" => {
            let pod: Pod = serde_yaml::from_str(&yaml_str)?;
            let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Pod = client
                .post(&format!("/api/v1/namespaces/{}/pods", namespace), &pod)
                .await?;
            println!("Pod '{}' created", pod.metadata.name);
        }
        "Service" => {
            let service: Service = serde_yaml::from_str(&yaml_str)?;
            let namespace = service.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Service = client
                .post(
                    &format!("/api/v1/namespaces/{}/services", namespace),
                    &service,
                )
                .await?;
            println!("Service '{}' created", service.metadata.name);
        }
        "Deployment" => {
            let deployment: Deployment = serde_yaml::from_str(&yaml_str)?;
            let namespace = deployment.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Deployment = client
                .post(
                    &format!("/apis/apps/v1/namespaces/{}/deployments", namespace),
                    &deployment,
                )
                .await?;
            println!("Deployment '{}' created", deployment.metadata.name);
        }
        "Node" => {
            let node: Node = serde_yaml::from_str(&yaml_str)?;
            let _result: Node = client.post("/api/v1/nodes", &node).await?;
            println!("Node '{}' created", node.metadata.name);
        }
        "Namespace" => {
            let namespace: Namespace = serde_yaml::from_str(&yaml_str)?;
            let _result: Namespace = client.post("/api/v1/namespaces", &namespace).await?;
            println!("Namespace '{}' created", namespace.metadata.name);
        }
        "StorageClass" => {
            let sc: StorageClass = serde_yaml::from_str(&yaml_str)?;
            let _result: StorageClass = client
                .post("/apis/storage.k8s.io/v1/storageclasses", &sc)
                .await?;
            println!("StorageClass '{}' created", sc.metadata.name);
        }
        "VolumeSnapshot" => {
            let vs: VolumeSnapshot = serde_yaml::from_str(&yaml_str)?;
            let namespace = vs.metadata.namespace.as_deref().unwrap_or("default");
            let _result: VolumeSnapshot = client
                .post(
                    &format!("/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots", namespace),
                    &vs,
                )
                .await?;
            println!("VolumeSnapshot '{}' created", vs.metadata.name);
        }
        "VolumeSnapshotClass" => {
            let vsc: VolumeSnapshotClass = serde_yaml::from_str(&yaml_str)?;
            let _result: VolumeSnapshotClass = client
                .post("/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses", &vsc)
                .await?;
            println!("VolumeSnapshotClass '{}' created", vsc.metadata.name);
        }
        "Endpoints" => {
            let ep: Endpoints = serde_yaml::from_str(&yaml_str)?;
            let namespace = ep.metadata.namespace.as_deref().unwrap_or("default");
            let _result: Endpoints = client
                .post(
                    &format!("/api/v1/namespaces/{}/endpoints", namespace),
                    &ep,
                )
                .await?;
            println!("Endpoints '{}' created", ep.metadata.name);
        }
        "ResourceQuota" => {
            let rq: ResourceQuota = serde_yaml::from_str(&yaml_str)?;
            let namespace = rq.metadata.namespace.as_deref().unwrap_or("default");
            let _result: ResourceQuota = client
                .post(
                    &format!("/api/v1/namespaces/{}/resourcequotas", namespace),
                    &rq,
                )
                .await?;
            println!("ResourceQuota '{}' created", rq.metadata.name);
        }
        "LimitRange" => {
            let lr: LimitRange = serde_yaml::from_str(&yaml_str)?;
            let namespace = lr.metadata.namespace.as_deref().unwrap_or("default");
            let _result: LimitRange = client
                .post(
                    &format!("/api/v1/namespaces/{}/limitranges", namespace),
                    &lr,
                )
                .await?;
            println!("LimitRange '{}' created", lr.metadata.name);
        }
        "PriorityClass" => {
            let pc: PriorityClass = serde_yaml::from_str(&yaml_str)?;
            let _result: PriorityClass = client
                .post("/apis/scheduling.k8s.io/v1/priorityclasses", &pc)
                .await?;
            println!("PriorityClass '{}' created", pc.metadata.name);
        }
        _ => anyhow::bail!("Unsupported resource kind: {}", kind),
    }

    Ok(())
}

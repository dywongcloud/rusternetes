use crate::client::ApiClient;
use anyhow::Result;
use rusternetes_common::resources::{Deployment, Namespace, Node, Pod, Service, PersistentVolume, PersistentVolumeClaim};

pub async fn execute(
    client: &ApiClient,
    resource_type: &str,
    name: Option<&str>,
    namespace: Option<&str>,
) -> Result<()> {
    let default_namespace = "default";
    let ns = namespace.unwrap_or(default_namespace);

    match resource_type {
        "pod" | "pods" => {
            if let Some(name) = name {
                let pod: Pod = client
                    .get(&format!("/api/v1/namespaces/{}/pods/{}", ns, name))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&pod)?);
            } else {
                let pods: Vec<Pod> = client
                    .get(&format!("/api/v1/namespaces/{}/pods", ns))
                    .await?;
                print_pods(&pods);
            }
        }
        "service" | "services" | "svc" => {
            if let Some(name) = name {
                let service: Service = client
                    .get(&format!("/api/v1/namespaces/{}/services/{}", ns, name))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&service)?);
            } else {
                let services: Vec<Service> = client
                    .get(&format!("/api/v1/namespaces/{}/services", ns))
                    .await?;
                print_services(&services);
            }
        }
        "deployment" | "deployments" | "deploy" => {
            if let Some(name) = name {
                let deployment: Deployment = client
                    .get(&format!(
                        "/apis/apps/v1/namespaces/{}/deployments/{}",
                        ns, name
                    ))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&deployment)?);
            } else {
                let deployments: Vec<Deployment> = client
                    .get(&format!("/apis/apps/v1/namespaces/{}/deployments", ns))
                    .await?;
                print_deployments(&deployments);
            }
        }
        "node" | "nodes" => {
            if let Some(name) = name {
                let node: Node = client.get(&format!("/api/v1/nodes/{}", name)).await?;
                println!("{}", serde_json::to_string_pretty(&node)?);
            } else {
                let nodes: Vec<Node> = client.get("/api/v1/nodes").await?;
                print_nodes(&nodes);
            }
        }
        "namespace" | "namespaces" | "ns" => {
            if let Some(name) = name {
                let namespace: Namespace = client.get(&format!("/api/v1/namespaces/{}", name)).await?;
                println!("{}", serde_json::to_string_pretty(&namespace)?);
            } else {
                let namespaces: Vec<Namespace> = client.get("/api/v1/namespaces").await?;
                print_namespaces(&namespaces);
            }
        }
        "persistentvolume" | "persistentvolumes" | "pv" => {
            if let Some(name) = name {
                let pv: PersistentVolume = client.get(&format!("/api/v1/persistentvolumes/{}", name)).await?;
                println!("{}", serde_json::to_string_pretty(&pv)?);
            } else {
                let pvs: Vec<PersistentVolume> = client.get("/api/v1/persistentvolumes").await?;
                print_pvs(&pvs);
            }
        }
        "persistentvolumeclaim" | "persistentvolumeclaims" | "pvc" => {
            if let Some(name) = name {
                let pvc: PersistentVolumeClaim = client.get(&format!("/api/v1/namespaces/{}/persistentvolumeclaims/{}", ns, name)).await?;
                println!("{}", serde_json::to_string_pretty(&pvc)?);
            } else {
                let pvcs: Vec<PersistentVolumeClaim> = client.get(&format!("/api/v1/namespaces/{}/persistentvolumeclaims", ns)).await?;
                print_pvcs(&pvcs);
            }
        }
        _ => anyhow::bail!("Unknown resource type: {}", resource_type),
    }

    Ok(())
}

fn print_pods(pods: &[Pod]) {
    println!("{:<30} {:<15} {:<15}", "NAME", "STATUS", "NODE");
    for pod in pods {
        let status = pod
            .status
            .as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        let node = pod
            .spec
            .node_name
            .as_ref()
            .map(|n| n.as_str())
            .unwrap_or("<none>");
        println!("{:<30} {:<15} {:<15}", pod.metadata.name, status, node);
    }
}

fn print_services(services: &[Service]) {
    println!("{:<30} {:<20} {:<10}", "NAME", "CLUSTER-IP", "PORTS");
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

fn print_deployments(deployments: &[Deployment]) {
    println!("{:<30} {:<10} {:<10}", "NAME", "DESIRED", "READY");
    for deployment in deployments {
        let desired = deployment.spec.replicas;
        let ready = deployment
            .status
            .as_ref()
            .and_then(|s| s.ready_replicas)
            .unwrap_or(0);
        println!(
            "{:<30} {:<10} {:<10}",
            deployment.metadata.name, desired, ready
        );
    }
}

fn print_nodes(nodes: &[Node]) {
    println!("{:<30} {:<15}", "NAME", "STATUS");
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

fn print_namespaces(namespaces: &[Namespace]) {
    println!("{:<30} {:<15}", "NAME", "STATUS");
    for namespace in namespaces {
        let status = namespace
            .status
            .as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        println!("{:<30} {:<15}", namespace.metadata.name, status);
    }
}

fn print_pvs(pvs: &[PersistentVolume]) {
    println!("{:<30} {:<15} {:<20} {:<15}", "NAME", "CAPACITY", "ACCESS MODES", "STATUS");
    for pv in pvs {
        let capacity = pv.spec.capacity.get("storage")
            .map(|s| s.as_str())
            .unwrap_or("<none>");
        let access_modes = pv.spec.access_modes.iter()
            .map(|m| format!("{:?}", m))
            .collect::<Vec<_>>()
            .join(",");
        let status = pv.status.as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        println!("{:<30} {:<15} {:<20} {:<15}", pv.metadata.name, capacity, access_modes, status);
    }
}

fn print_pvcs(pvcs: &[PersistentVolumeClaim]) {
    println!("{:<30} {:<15} {:<20} {:<20} {:<15}", "NAME", "STATUS", "VOLUME", "CAPACITY", "ACCESS MODES");
    for pvc in pvcs {
        let status = pvc.status.as_ref()
            .map(|s| format!("{:?}", s.phase))
            .unwrap_or_else(|| "Unknown".to_string());
        let volume = pvc.spec.volume_name.as_deref().unwrap_or("<none>");
        let capacity = pvc.status.as_ref()
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("storage"))
            .map(|s| s.as_str())
            .unwrap_or("<none>");
        let access_modes = pvc.spec.access_modes.iter()
            .map(|m| format!("{:?}", m))
            .collect::<Vec<_>>()
            .join(",");
        println!("{:<30} {:<15} {:<20} {:<20} {:<15}", pvc.metadata.name, status, volume, capacity, access_modes);
    }
}

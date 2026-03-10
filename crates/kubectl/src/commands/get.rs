use crate::client::{ApiClient, GetError};
use anyhow::Result;
use rusternetes_common::resources::{
    Deployment, Namespace, Node, Pod, Service, PersistentVolume, PersistentVolumeClaim,
    StorageClass, VolumeSnapshot, VolumeSnapshotClass, Endpoints, ConfigMap, Secret,
    StatefulSet, DaemonSet, Ingress, ServiceAccount, Role, RoleBinding, ClusterRole, ClusterRoleBinding,
    ResourceQuota, LimitRange, PriorityClass, CustomResourceDefinition,
};
use serde::Serialize;

// Helper to convert GetError to anyhow::Error
fn map_get_error(err: GetError) -> anyhow::Error {
    match err {
        GetError::NotFound => anyhow::anyhow!("Resource not found"),
        GetError::Other(e) => e,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum OutputFormat {
    Table,
    Json,
    Yaml,
    Wide,
}

impl OutputFormat {
    pub(crate) fn from_str(s: &str) -> Result<Self> {
        match s {
            "json" => Ok(Self::Json),
            "yaml" => Ok(Self::Yaml),
            "wide" => Ok(Self::Wide),
            _ => anyhow::bail!("Unknown output format: {}. Supported formats: json, yaml, wide", s),
        }
    }
}

pub(crate) fn format_output<T: Serialize>(resource: &T, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(resource)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(resource)?);
        }
        _ => {
            // For table and wide, just use JSON for now
            println!("{}", serde_json::to_string_pretty(resource)?);
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
) -> Result<()> {
    let default_namespace = "default";
    let ns = namespace.unwrap_or(default_namespace);
    let format = output.map(OutputFormat::from_str).transpose()?.unwrap_or(OutputFormat::Table);

    match resource_type {
        "pod" | "pods" => {
            if let Some(name) = name {
                let pod: Pod = client
                    .get(&format!("/api/v1/namespaces/{}/pods/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&pod, format)?;
            } else {
                let pods: Vec<Pod> = client
                    .get(&format!("/api/v1/namespaces/{}/pods", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_pods(&pods),
                    OutputFormat::Json => format_output(&pods, format)?,
                    OutputFormat::Yaml => format_output(&pods, format)?,
                }
            }
        }
        "service" | "services" | "svc" => {
            if let Some(name) = name {
                let service: Service = client
                    .get(&format!("/api/v1/namespaces/{}/services/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&service, format)?;
            } else {
                let services: Vec<Service> = client
                    .get(&format!("/api/v1/namespaces/{}/services", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_services(&services),
                    OutputFormat::Json => format_output(&services, format)?,
                    OutputFormat::Yaml => format_output(&services, format)?,
                }
            }
        }
        "deployment" | "deployments" | "deploy" => {
            if let Some(name) = name {
                let deployment: Deployment = client
                    .get(&format!("/apis/apps/v1/namespaces/{}/deployments/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&deployment, format)?;
            } else {
                let deployments: Vec<Deployment> = client
                    .get(&format!("/apis/apps/v1/namespaces/{}/deployments", ns))
                    .await
                    .map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_deployments(&deployments),
                    OutputFormat::Json => format_output(&deployments, format)?,
                    OutputFormat::Yaml => format_output(&deployments, format)?,
                }
            }
        }
        "statefulset" | "statefulsets" | "sts" => {
            if let Some(name) = name {
                let statefulset: StatefulSet = client
                    .get(&format!("/apis/apps/v1/namespaces/{}/statefulsets/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&statefulset, format)?;
            } else {
                let statefulsets: Vec<StatefulSet> = client
                    .get(&format!("/apis/apps/v1/namespaces/{}/statefulsets", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&statefulsets, format)?;
            }
        }
        "daemonset" | "daemonsets" | "ds" => {
            if let Some(name) = name {
                let daemonset: DaemonSet = client
                    .get(&format!("/apis/apps/v1/namespaces/{}/daemonsets/{}", ns, name))
                    .await
                    .map_err(map_get_error)?;
                format_output(&daemonset, format)?;
            } else {
                let daemonsets: Vec<DaemonSet> = client
                    .get(&format!("/apis/apps/v1/namespaces/{}/daemonsets", ns))
                    .await
                    .map_err(map_get_error)?;
                format_output(&daemonsets, format)?;
            }
        }
        "node" | "nodes" => {
            if let Some(name) = name {
                let node: Node = client.get(&format!("/api/v1/nodes/{}", name)).await.map_err(map_get_error)?;
                format_output(&node, format)?;
            } else {
                let nodes: Vec<Node> = client.get("/api/v1/nodes").await.map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_nodes(&nodes),
                    OutputFormat::Json => format_output(&nodes, format)?,
                    OutputFormat::Yaml => format_output(&nodes, format)?,
                }
            }
        }
        "namespace" | "namespaces" | "ns" => {
            if let Some(name) = name {
                let namespace: Namespace = client.get(&format!("/api/v1/namespaces/{}", name)).await.map_err(map_get_error)?;
                format_output(&namespace, format)?;
            } else {
                let namespaces: Vec<Namespace> = client.get("/api/v1/namespaces").await.map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_namespaces(&namespaces),
                    OutputFormat::Json => format_output(&namespaces, format)?,
                    OutputFormat::Yaml => format_output(&namespaces, format)?,
                }
            }
        }
        "persistentvolume" | "persistentvolumes" | "pv" => {
            if let Some(name) = name {
                let pv: PersistentVolume = client.get(&format!("/api/v1/persistentvolumes/{}", name)).await.map_err(map_get_error)?;
                format_output(&pv, format)?;
            } else {
                let pvs: Vec<PersistentVolume> = client.get("/api/v1/persistentvolumes").await.map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_pvs(&pvs),
                    OutputFormat::Json => format_output(&pvs, format)?,
                    OutputFormat::Yaml => format_output(&pvs, format)?,
                }
            }
        }
        "persistentvolumeclaim" | "persistentvolumeclaims" | "pvc" => {
            if let Some(name) = name {
                let pvc: PersistentVolumeClaim = client.get(&format!("/api/v1/namespaces/{}/persistentvolumeclaims/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&pvc, format)?;
            } else {
                let pvcs: Vec<PersistentVolumeClaim> = client.get(&format!("/api/v1/namespaces/{}/persistentvolumeclaims", ns)).await.map_err(map_get_error)?;
                match format {
                    OutputFormat::Table | OutputFormat::Wide => print_pvcs(&pvcs),
                    OutputFormat::Json => format_output(&pvcs, format)?,
                    OutputFormat::Yaml => format_output(&pvcs, format)?,
                }
            }
        }
        "storageclass" | "storageclasses" | "sc" => {
            if let Some(name) = name {
                let sc: StorageClass = client.get(&format!("/apis/storage.k8s.io/v1/storageclasses/{}", name)).await.map_err(map_get_error)?;
                format_output(&sc, format)?;
            } else {
                let scs: Vec<StorageClass> = client.get("/apis/storage.k8s.io/v1/storageclasses").await.map_err(map_get_error)?;
                format_output(&scs, format)?;
            }
        }
        "volumesnapshot" | "volumesnapshots" | "vs" => {
            if let Some(name) = name {
                let vs: VolumeSnapshot = client.get(&format!("/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&vs, format)?;
            } else {
                let vss: Vec<VolumeSnapshot> = client.get(&format!("/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots", ns)).await.map_err(map_get_error)?;
                format_output(&vss, format)?;
            }
        }
        "volumesnapshotclass" | "volumesnapshotclasses" | "vsc" => {
            if let Some(name) = name {
                let vsc: VolumeSnapshotClass = client.get(&format!("/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/{}", name)).await.map_err(map_get_error)?;
                format_output(&vsc, format)?;
            } else {
                let vscs: Vec<VolumeSnapshotClass> = client.get("/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses").await.map_err(map_get_error)?;
                format_output(&vscs, format)?;
            }
        }
        "endpoints" | "ep" => {
            if let Some(name) = name {
                let ep: Endpoints = client.get(&format!("/api/v1/namespaces/{}/endpoints/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&ep, format)?;
            } else {
                let eps: Vec<Endpoints> = client.get(&format!("/api/v1/namespaces/{}/endpoints", ns)).await.map_err(map_get_error)?;
                format_output(&eps, format)?;
            }
        }
        "configmap" | "configmaps" | "cm" => {
            if let Some(name) = name {
                let cm: ConfigMap = client.get(&format!("/api/v1/namespaces/{}/configmaps/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&cm, format)?;
            } else {
                let cms: Vec<ConfigMap> = client.get(&format!("/api/v1/namespaces/{}/configmaps", ns)).await.map_err(map_get_error)?;
                format_output(&cms, format)?;
            }
        }
        "secret" | "secrets" => {
            if let Some(name) = name {
                let secret: Secret = client.get(&format!("/api/v1/namespaces/{}/secrets/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&secret, format)?;
            } else {
                let secrets: Vec<Secret> = client.get(&format!("/api/v1/namespaces/{}/secrets", ns)).await.map_err(map_get_error)?;
                format_output(&secrets, format)?;
            }
        }
        "ingress" | "ingresses" | "ing" => {
            if let Some(name) = name {
                let ing: Ingress = client.get(&format!("/apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&ing, format)?;
            } else {
                let ings: Vec<Ingress> = client.get(&format!("/apis/networking.k8s.io/v1/namespaces/{}/ingresses", ns)).await.map_err(map_get_error)?;
                format_output(&ings, format)?;
            }
        }
        "serviceaccount" | "serviceaccounts" | "sa" => {
            if let Some(name) = name {
                let sa: ServiceAccount = client.get(&format!("/api/v1/namespaces/{}/serviceaccounts/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&sa, format)?;
            } else {
                let sas: Vec<ServiceAccount> = client.get(&format!("/api/v1/namespaces/{}/serviceaccounts", ns)).await.map_err(map_get_error)?;
                format_output(&sas, format)?;
            }
        }
        "role" | "roles" => {
            if let Some(name) = name {
                let role: Role = client.get(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&role, format)?;
            } else {
                let roles: Vec<Role> = client.get(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles", ns)).await.map_err(map_get_error)?;
                format_output(&roles, format)?;
            }
        }
        "rolebinding" | "rolebindings" => {
            if let Some(name) = name {
                let rb: RoleBinding = client.get(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&rb, format)?;
            } else {
                let rbs: Vec<RoleBinding> = client.get(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings", ns)).await.map_err(map_get_error)?;
                format_output(&rbs, format)?;
            }
        }
        "clusterrole" | "clusterroles" => {
            if let Some(name) = name {
                let cr: ClusterRole = client.get(&format!("/apis/rbac.authorization.k8s.io/v1/clusterroles/{}", name)).await.map_err(map_get_error)?;
                format_output(&cr, format)?;
            } else {
                let crs: Vec<ClusterRole> = client.get("/apis/rbac.authorization.k8s.io/v1/clusterroles").await.map_err(map_get_error)?;
                format_output(&crs, format)?;
            }
        }
        "clusterrolebinding" | "clusterrolebindings" => {
            if let Some(name) = name {
                let crb: ClusterRoleBinding = client.get(&format!("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}", name)).await.map_err(map_get_error)?;
                format_output(&crb, format)?;
            } else {
                let crbs: Vec<ClusterRoleBinding> = client.get("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings").await.map_err(map_get_error)?;
                format_output(&crbs, format)?;
            }
        }
        "resourcequota" | "resourcequotas" | "quota" => {
            if let Some(name) = name {
                let rq: ResourceQuota = client.get(&format!("/api/v1/namespaces/{}/resourcequotas/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&rq, format)?;
            } else {
                let rqs: Vec<ResourceQuota> = client.get(&format!("/api/v1/namespaces/{}/resourcequotas", ns)).await.map_err(map_get_error)?;
                format_output(&rqs, format)?;
            }
        }
        "limitrange" | "limitranges" | "limits" => {
            if let Some(name) = name {
                let lr: LimitRange = client.get(&format!("/api/v1/namespaces/{}/limitranges/{}", ns, name)).await.map_err(map_get_error)?;
                format_output(&lr, format)?;
            } else {
                let lrs: Vec<LimitRange> = client.get(&format!("/api/v1/namespaces/{}/limitranges", ns)).await.map_err(map_get_error)?;
                format_output(&lrs, format)?;
            }
        }
        "priorityclass" | "priorityclasses" | "pc" => {
            if let Some(name) = name {
                let pc: PriorityClass = client.get(&format!("/apis/scheduling.k8s.io/v1/priorityclasses/{}", name)).await.map_err(map_get_error)?;
                format_output(&pc, format)?;
            } else {
                let pcs: Vec<PriorityClass> = client.get("/apis/scheduling.k8s.io/v1/priorityclasses").await.map_err(map_get_error)?;
                format_output(&pcs, format)?;
            }
        }
        "customresourcedefinition" | "customresourcedefinitions" | "crd" | "crds" => {
            if let Some(name) = name {
                let crd: CustomResourceDefinition = client.get(&format!("/apis/apiextensions.k8s.io/v1/customresourcedefinitions/{}", name)).await.map_err(map_get_error)?;
                format_output(&crd, format)?;
            } else {
                let crds: Vec<CustomResourceDefinition> = client.get("/apis/apiextensions.k8s.io/v1/customresourcedefinitions").await.map_err(map_get_error)?;
                format_output(&crds, format)?;
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
            .as_ref()
            .and_then(|s| s.node_name.as_ref())
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

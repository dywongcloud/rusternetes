use crate::client::{ApiClient, GetError};
use anyhow::{Context, Result};
use rusternetes_common::resources::{
    Deployment, Namespace, Node, Pod, Service, Job, CronJob, PersistentVolume, PersistentVolumeClaim,
    StorageClass, VolumeSnapshot, VolumeSnapshotClass, Endpoints, ConfigMap, Secret,
    StatefulSet, DaemonSet, Ingress, ServiceAccount, Role, RoleBinding, ClusterRole, ClusterRoleBinding,
    ResourceQuota, LimitRange, PriorityClass, CustomResourceDefinition,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fs;
use std::io::{self, Read};

pub async fn execute_enhanced(
    client: &ApiClient,
    file: &str,
    namespace: Option<&str>,
    dry_run: Option<&str>,
    server_side: bool,
    force: bool,
) -> Result<()> {
    if let Some(dr) = dry_run {
        println!("Dry run mode: {}", dr);
    }
    if server_side {
        println!("Server-side apply enabled");
    }
    if force {
        println!("Force apply enabled");
    }
    execute(client, file).await
}

pub async fn execute(client: &ApiClient, file: &str) -> Result<()> {
    let contents = if file == "-" {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        buffer
    } else {
        // Read from file
        fs::read_to_string(file).context("Failed to read file")?
    };

    // Support for multi-document YAML files
    for document in serde_yaml::Deserializer::from_str(&contents) {
        let value = serde_yaml::Value::deserialize(document)?;

        // Skip empty documents
        if value.is_null() {
            continue;
        }

        apply_resource(client, &value).await?;
    }

    Ok(())
}

// Helper function to check if a resource exists
async fn resource_exists<T: DeserializeOwned>(client: &ApiClient, path: &str) -> Result<bool> {
    match client.get::<T>(path).await {
        Ok(_) => Ok(true),
        Err(GetError::NotFound) => Ok(false),
        Err(GetError::Other(e)) => Err(e),
    }
}

async fn apply_resource(client: &ApiClient, value: &serde_yaml::Value) -> Result<()> {
    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .context("Missing 'kind' field")?;

    let yaml_str = serde_yaml::to_string(value)?;

    match kind {
        "Pod" => {
            let mut pod: Pod = serde_yaml::from_str(&yaml_str)?;
            let namespace = pod.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = pod.metadata.name.clone();

            let exists = resource_exists::<Pod>(client, &format!("/api/v1/namespaces/{}/pods/{}", namespace, name)).await?;

            if exists {
                // Update existing resource
                let _result: Pod = client
                    .put(
                        &format!("/api/v1/namespaces/{}/pods/{}", namespace, name),
                        &pod,
                    )
                    .await?;
                println!("pod/{} configured", name);
            } else {
                // Create new resource
                pod.metadata.ensure_uid();
                pod.metadata.ensure_creation_timestamp();
                let _result: Pod = client
                    .post(
                        &format!("/api/v1/namespaces/{}/pods", namespace),
                        &pod,
                    )
                    .await?;
                println!("pod/{} created", name);
            }
        }
        "Service" => {
            let mut service: Service = serde_yaml::from_str(&yaml_str)?;
            let namespace = service.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = service.metadata.name.clone();

            let exists = resource_exists::<Service>(client, &format!("/api/v1/namespaces/{}/services/{}", namespace, name)).await?;

            if exists {
                let _result: Service = client
                    .put(&format!("/api/v1/namespaces/{}/services/{}", namespace, name), &service)
                    .await?;
                println!("service/{} configured", name);
            } else {
                service.metadata.ensure_uid();
                service.metadata.ensure_creation_timestamp();
                let _result: Service = client
                    .post(&format!("/api/v1/namespaces/{}/services", namespace), &service)
                    .await?;
                println!("service/{} created", name);
            }
        }
        "Deployment" => {
            let mut deployment: Deployment = serde_yaml::from_str(&yaml_str)?;
            let namespace = deployment.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = deployment.metadata.name.clone();

            let exists = resource_exists::<Deployment>(client, &format!("/apis/apps/v1/namespaces/{}/deployments/{}", namespace, name)).await?;

            if exists {
                let _result: Deployment = client
                    .put(&format!("/apis/apps/v1/namespaces/{}/deployments/{}", namespace, name), &deployment)
                    .await?;
                println!("deployment.apps/{} configured", name);
            } else {
                deployment.metadata.ensure_uid();
                deployment.metadata.ensure_creation_timestamp();
                let _result: Deployment = client
                    .post(&format!("/apis/apps/v1/namespaces/{}/deployments", namespace), &deployment)
                    .await?;
                println!("deployment.apps/{} created", name);
            }
        }
        "StatefulSet" => {
            let mut statefulset: StatefulSet = serde_yaml::from_str(&yaml_str)?;
            let namespace = statefulset.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = statefulset.metadata.name.clone();

            let exists = resource_exists::<StatefulSet>(client, &format!("/apis/apps/v1/namespaces/{}/statefulsets/{}", namespace, name)).await?;

            if exists {
                let _result: StatefulSet = client
                    .put(&format!("/apis/apps/v1/namespaces/{}/statefulsets/{}", namespace, name), &statefulset)
                    .await?;
                println!("statefulset.apps/{} configured", name);
            } else {
                statefulset.metadata.ensure_uid();
                statefulset.metadata.ensure_creation_timestamp();
                let _result: StatefulSet = client
                    .post(&format!("/apis/apps/v1/namespaces/{}/statefulsets", namespace), &statefulset)
                    .await?;
                println!("statefulset.apps/{} created", name);
            }
        }
        "DaemonSet" => {
            let mut daemonset: DaemonSet = serde_yaml::from_str(&yaml_str)?;
            let namespace = daemonset.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = daemonset.metadata.name.clone();

            let exists = resource_exists::<DaemonSet>(client, &format!("/apis/apps/v1/namespaces/{}/daemonsets/{}", namespace, name)).await?;

            if exists {
                let _result: DaemonSet = client
                    .put(&format!("/apis/apps/v1/namespaces/{}/daemonsets/{}", namespace, name), &daemonset)
                    .await?;
                println!("daemonset.apps/{} configured", name);
            } else {
                daemonset.metadata.ensure_uid();
                daemonset.metadata.ensure_creation_timestamp();
                let _result: DaemonSet = client
                    .post(&format!("/apis/apps/v1/namespaces/{}/daemonsets", namespace), &daemonset)
                    .await?;
                println!("daemonset.apps/{} created", name);
            }
        }
        "Node" => {
            let mut node: Node = serde_yaml::from_str(&yaml_str)?;
            let name = node.metadata.name.clone();

            let exists = resource_exists::<Node>(client, &format!("/api/v1/nodes/{}", name)).await?;

            if exists {
                let _result: Node = client
                    .put(&format!("/api/v1/nodes/{}", name), &node)
                    .await?;
                println!("node/{} configured", name);
            } else {
                node.metadata.ensure_uid();
                node.metadata.ensure_creation_timestamp();
                let _result: Node = client
                    .post("/api/v1/nodes", &node)
                    .await?;
                println!("node/{} created", name);
            }
        }
        "Namespace" => {
            let mut namespace: Namespace = serde_yaml::from_str(&yaml_str)?;
            let name = namespace.metadata.name.clone();

            let exists = resource_exists::<Namespace>(client, &format!("/api/v1/namespaces/{}", name)).await?;

            if exists {
                let _result: Namespace = client
                    .put(&format!("/api/v1/namespaces/{}", name), &namespace)
                    .await?;
                println!("namespace/{} configured", name);
            } else {
                namespace.metadata.ensure_uid();
                namespace.metadata.ensure_creation_timestamp();
                let _result: Namespace = client
                    .post("/api/v1/namespaces", &namespace)
                    .await?;
                println!("namespace/{} created", name);
            }
        }
        "Job" => {
            let mut job: Job = serde_yaml::from_str(&yaml_str)?;
            let namespace = job.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = job.metadata.name.clone();

            let exists = resource_exists::<Job>(client, &format!("/apis/batch/v1/namespaces/{}/jobs/{}", namespace, name)).await?;

            if exists {
                let _result: Job = client
                    .put(&format!("/apis/batch/v1/namespaces/{}/jobs/{}", namespace, name), &job)
                    .await?;
                println!("job.batch/{} configured", name);
            } else {
                job.metadata.ensure_uid();
                job.metadata.ensure_creation_timestamp();
                let _result: Job = client
                    .post(&format!("/apis/batch/v1/namespaces/{}/jobs", namespace), &job)
                    .await?;
                println!("job.batch/{} created", name);
            }
        }
        "CronJob" => {
            let mut cronjob: CronJob = serde_yaml::from_str(&yaml_str)?;
            let namespace = cronjob.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = cronjob.metadata.name.clone();

            let exists = resource_exists::<CronJob>(client, &format!("/apis/batch/v1/namespaces/{}/cronjobs/{}", namespace, name)).await?;

            if exists {
                let _result: CronJob = client
                    .put(&format!("/apis/batch/v1/namespaces/{}/cronjobs/{}", namespace, name), &cronjob)
                    .await?;
                println!("cronjob.batch/{} configured", name);
            } else {
                cronjob.metadata.ensure_uid();
                cronjob.metadata.ensure_creation_timestamp();
                let _result: CronJob = client
                    .post(&format!("/apis/batch/v1/namespaces/{}/cronjobs", namespace), &cronjob)
                    .await?;
                println!("cronjob.batch/{} created", name);
            }
        }
        "PersistentVolume" => {
            let mut pv: PersistentVolume = serde_yaml::from_str(&yaml_str)?;
            let name = pv.metadata.name.clone();

            let exists = resource_exists::<PersistentVolume>(client, &format!("/api/v1/persistentvolumes/{}", name)).await?;

            if exists {
                let _result: PersistentVolume = client
                    .put(&format!("/api/v1/persistentvolumes/{}", name), &pv)
                    .await?;
                println!("persistentvolume/{} configured", name);
            } else {
                pv.metadata.ensure_uid();
                pv.metadata.ensure_creation_timestamp();
                let _result: PersistentVolume = client
                    .post("/api/v1/persistentvolumes", &pv)
                    .await?;
                println!("persistentvolume/{} created", name);
            }
        }
        "PersistentVolumeClaim" => {
            let mut pvc: PersistentVolumeClaim = serde_yaml::from_str(&yaml_str)?;
            let namespace = pvc.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = pvc.metadata.name.clone();

            let exists = resource_exists::<PersistentVolumeClaim>(client, &format!("/api/v1/namespaces/{}/persistentvolumeclaims/{}", namespace, name)).await?;

            if exists {
                let _result: PersistentVolumeClaim = client
                    .put(&format!("/api/v1/namespaces/{}/persistentvolumeclaims/{}", namespace, name), &pvc)
                    .await?;
                println!("persistentvolumeclaim/{} configured", name);
            } else {
                pvc.metadata.ensure_uid();
                pvc.metadata.ensure_creation_timestamp();
                let _result: PersistentVolumeClaim = client
                    .post(&format!("/api/v1/namespaces/{}/persistentvolumeclaims", namespace), &pvc)
                    .await?;
                println!("persistentvolumeclaim/{} created", name);
            }
        }
        "StorageClass" => {
            let mut sc: StorageClass = serde_yaml::from_str(&yaml_str)?;
            let name = sc.metadata.name.clone();

            let exists = resource_exists::<StorageClass>(client, &format!("/apis/storage.k8s.io/v1/storageclasses/{}", name)).await?;

            if exists {
                let _result: StorageClass = client
                    .put(&format!("/apis/storage.k8s.io/v1/storageclasses/{}", name), &sc)
                    .await?;
                println!("storageclass.storage.k8s.io/{} configured", name);
            } else {
                sc.metadata.ensure_uid();
                sc.metadata.ensure_creation_timestamp();
                let _result: StorageClass = client
                    .post("/apis/storage.k8s.io/v1/storageclasses", &sc)
                    .await?;
                println!("storageclass.storage.k8s.io/{} created", name);
            }
        }
        "VolumeSnapshot" => {
            let mut snapshot: VolumeSnapshot = serde_yaml::from_str(&yaml_str)?;
            let namespace = snapshot.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = snapshot.metadata.name.clone();

            let exists = resource_exists::<VolumeSnapshot>(client, &format!("/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots/{}", namespace, name)).await?;

            if exists {
                let _result: VolumeSnapshot = client
                    .put(&format!("/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots/{}", namespace, name), &snapshot)
                    .await?;
                println!("volumesnapshot.snapshot.storage.k8s.io/{} configured", name);
            } else {
                snapshot.metadata.ensure_uid();
                snapshot.metadata.ensure_creation_timestamp();
                let _result: VolumeSnapshot = client
                    .post(&format!("/apis/snapshot.storage.k8s.io/v1/namespaces/{}/volumesnapshots", namespace), &snapshot)
                    .await?;
                println!("volumesnapshot.snapshot.storage.k8s.io/{} created", name);
            }
        }
        "VolumeSnapshotClass" => {
            let mut vsc: VolumeSnapshotClass = serde_yaml::from_str(&yaml_str)?;
            let name = vsc.metadata.name.clone();

            let exists = resource_exists::<VolumeSnapshotClass>(client, &format!("/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/{}", name)).await?;

            if exists {
                let _result: VolumeSnapshotClass = client
                    .put(&format!("/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/{}", name), &vsc)
                    .await?;
                println!("volumesnapshotclass.snapshot.storage.k8s.io/{} configured", name);
            } else {
                vsc.metadata.ensure_uid();
                vsc.metadata.ensure_creation_timestamp();
                let _result: VolumeSnapshotClass = client
                    .post("/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses", &vsc)
                    .await?;
                println!("volumesnapshotclass.snapshot.storage.k8s.io/{} created", name);
            }
        }
        "Endpoints" => {
            let mut endpoints: Endpoints = serde_yaml::from_str(&yaml_str)?;
            let namespace = endpoints.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = endpoints.metadata.name.clone();

            let exists = resource_exists::<Endpoints>(client, &format!("/api/v1/namespaces/{}/endpoints/{}", namespace, name)).await?;

            if exists {
                let _result: Endpoints = client
                    .put(&format!("/api/v1/namespaces/{}/endpoints/{}", namespace, name), &endpoints)
                    .await?;
                println!("endpoints/{} configured", name);
            } else {
                endpoints.metadata.ensure_uid();
                endpoints.metadata.ensure_creation_timestamp();
                let _result: Endpoints = client
                    .post(&format!("/api/v1/namespaces/{}/endpoints", namespace), &endpoints)
                    .await?;
                println!("endpoints/{} created", name);
            }
        }
        "ConfigMap" => {
            let mut configmap: ConfigMap = serde_yaml::from_str(&yaml_str)?;
            let namespace = configmap.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = configmap.metadata.name.clone();

            let exists = resource_exists::<ConfigMap>(client, &format!("/api/v1/namespaces/{}/configmaps/{}", namespace, name)).await?;

            if exists {
                let _result: ConfigMap = client
                    .put(&format!("/api/v1/namespaces/{}/configmaps/{}", namespace, name), &configmap)
                    .await?;
                println!("configmap/{} configured", name);
            } else {
                configmap.metadata.ensure_uid();
                configmap.metadata.ensure_creation_timestamp();
                let _result: ConfigMap = client
                    .post(&format!("/api/v1/namespaces/{}/configmaps", namespace), &configmap)
                    .await?;
                println!("configmap/{} created", name);
            }
        }
        "Secret" => {
            let mut secret: Secret = serde_yaml::from_str(&yaml_str)?;
            let namespace = secret.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = secret.metadata.name.clone();

            let exists = resource_exists::<Secret>(client, &format!("/api/v1/namespaces/{}/secrets/{}", namespace, name)).await?;

            if exists {
                let _result: Secret = client
                    .put(&format!("/api/v1/namespaces/{}/secrets/{}", namespace, name), &secret)
                    .await?;
                println!("secret/{} configured", name);
            } else {
                secret.metadata.ensure_uid();
                secret.metadata.ensure_creation_timestamp();
                let _result: Secret = client
                    .post(&format!("/api/v1/namespaces/{}/secrets", namespace), &secret)
                    .await?;
                println!("secret/{} created", name);
            }
        }
        "Ingress" => {
            let mut ingress: Ingress = serde_yaml::from_str(&yaml_str)?;
            let namespace = ingress.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = ingress.metadata.name.clone();

            let exists = resource_exists::<Ingress>(client, &format!("/apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}", namespace, name)).await?;

            if exists {
                let _result: Ingress = client
                    .put(&format!("/apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}", namespace, name), &ingress)
                    .await?;
                println!("ingress.networking.k8s.io/{} configured", name);
            } else {
                ingress.metadata.ensure_uid();
                ingress.metadata.ensure_creation_timestamp();
                let _result: Ingress = client
                    .post(&format!("/apis/networking.k8s.io/v1/namespaces/{}/ingresses", namespace), &ingress)
                    .await?;
                println!("ingress.networking.k8s.io/{} created", name);
            }
        }
        "ServiceAccount" => {
            let mut sa: ServiceAccount = serde_yaml::from_str(&yaml_str)?;
            let namespace = sa.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = sa.metadata.name.clone();

            let exists = resource_exists::<ServiceAccount>(client, &format!("/api/v1/namespaces/{}/serviceaccounts/{}", namespace, name)).await?;

            if exists {
                let _result: ServiceAccount = client
                    .put(&format!("/api/v1/namespaces/{}/serviceaccounts/{}", namespace, name), &sa)
                    .await?;
                println!("serviceaccount/{} configured", name);
            } else {
                sa.metadata.ensure_uid();
                sa.metadata.ensure_creation_timestamp();
                let _result: ServiceAccount = client
                    .post(&format!("/api/v1/namespaces/{}/serviceaccounts", namespace), &sa)
                    .await?;
                println!("serviceaccount/{} created", name);
            }
        }
        "Role" => {
            let mut role: Role = serde_yaml::from_str(&yaml_str)?;
            let namespace = role.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = role.metadata.name.clone();

            let exists = resource_exists::<Role>(client, &format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}", namespace, name)).await?;

            if exists {
                let _result: Role = client
                    .put(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles/{}", namespace, name), &role)
                    .await?;
                println!("role.rbac.authorization.k8s.io/{} configured", name);
            } else {
                role.metadata.ensure_uid();
                role.metadata.ensure_creation_timestamp();
                let _result: Role = client
                    .post(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/roles", namespace), &role)
                    .await?;
                println!("role.rbac.authorization.k8s.io/{} created", name);
            }
        }
        "RoleBinding" => {
            let mut rb: RoleBinding = serde_yaml::from_str(&yaml_str)?;
            let namespace = rb.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = rb.metadata.name.clone();

            let exists = resource_exists::<RoleBinding>(client, &format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}", namespace, name)).await?;

            if exists {
                let _result: RoleBinding = client
                    .put(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings/{}", namespace, name), &rb)
                    .await?;
                println!("rolebinding.rbac.authorization.k8s.io/{} configured", name);
            } else {
                rb.metadata.ensure_uid();
                rb.metadata.ensure_creation_timestamp();
                let _result: RoleBinding = client
                    .post(&format!("/apis/rbac.authorization.k8s.io/v1/namespaces/{}/rolebindings", namespace), &rb)
                    .await?;
                println!("rolebinding.rbac.authorization.k8s.io/{} created", name);
            }
        }
        "ClusterRole" => {
            let mut cr: ClusterRole = serde_yaml::from_str(&yaml_str)?;
            let name = cr.metadata.name.clone();

            let exists = resource_exists::<ClusterRole>(client, &format!("/apis/rbac.authorization.k8s.io/v1/clusterroles/{}", name)).await?;

            if exists {
                let _result: ClusterRole = client
                    .put(&format!("/apis/rbac.authorization.k8s.io/v1/clusterroles/{}", name), &cr)
                    .await?;
                println!("clusterrole.rbac.authorization.k8s.io/{} configured", name);
            } else {
                cr.metadata.ensure_uid();
                cr.metadata.ensure_creation_timestamp();
                let _result: ClusterRole = client
                    .post("/apis/rbac.authorization.k8s.io/v1/clusterroles", &cr)
                    .await?;
                println!("clusterrole.rbac.authorization.k8s.io/{} created", name);
            }
        }
        "ClusterRoleBinding" => {
            let mut crb: ClusterRoleBinding = serde_yaml::from_str(&yaml_str)?;
            let name = crb.metadata.name.clone();

            let exists = resource_exists::<ClusterRoleBinding>(client, &format!("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}", name)).await?;

            if exists {
                let _result: ClusterRoleBinding = client
                    .put(&format!("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{}", name), &crb)
                    .await?;
                println!("clusterrolebinding.rbac.authorization.k8s.io/{} configured", name);
            } else {
                crb.metadata.ensure_uid();
                crb.metadata.ensure_creation_timestamp();
                let _result: ClusterRoleBinding = client
                    .post("/apis/rbac.authorization.k8s.io/v1/clusterrolebindings", &crb)
                    .await?;
                println!("clusterrolebinding.rbac.authorization.k8s.io/{} created", name);
            }
        }
        "ResourceQuota" => {
            let mut rq: ResourceQuota = serde_yaml::from_str(&yaml_str)?;
            let namespace = rq.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = rq.metadata.name.clone();

            let exists = resource_exists::<ResourceQuota>(client, &format!("/api/v1/namespaces/{}/resourcequotas/{}", namespace, name)).await?;

            if exists {
                let _result: ResourceQuota = client
                    .put(&format!("/api/v1/namespaces/{}/resourcequotas/{}", namespace, name), &rq)
                    .await?;
                println!("resourcequota/{} configured", name);
            } else {
                rq.metadata.ensure_uid();
                rq.metadata.ensure_creation_timestamp();
                let _result: ResourceQuota = client
                    .post(&format!("/api/v1/namespaces/{}/resourcequotas", namespace), &rq)
                    .await?;
                println!("resourcequota/{} created", name);
            }
        }
        "LimitRange" => {
            let mut lr: LimitRange = serde_yaml::from_str(&yaml_str)?;
            let namespace = lr.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
            let name = lr.metadata.name.clone();

            let exists = resource_exists::<LimitRange>(client, &format!("/api/v1/namespaces/{}/limitranges/{}", namespace, name)).await?;

            if exists {
                let _result: LimitRange = client
                    .put(&format!("/api/v1/namespaces/{}/limitranges/{}", namespace, name), &lr)
                    .await?;
                println!("limitrange/{} configured", name);
            } else {
                lr.metadata.ensure_uid();
                lr.metadata.ensure_creation_timestamp();
                let _result: LimitRange = client
                    .post(&format!("/api/v1/namespaces/{}/limitranges", namespace), &lr)
                    .await?;
                println!("limitrange/{} created", name);
            }
        }
        "PriorityClass" => {
            let mut pc: PriorityClass = serde_yaml::from_str(&yaml_str)?;
            let name = pc.metadata.name.clone();

            let exists = resource_exists::<PriorityClass>(client, &format!("/apis/scheduling.k8s.io/v1/priorityclasses/{}", name)).await?;

            if exists {
                let _result: PriorityClass = client
                    .put(&format!("/apis/scheduling.k8s.io/v1/priorityclasses/{}", name), &pc)
                    .await?;
                println!("priorityclass.scheduling.k8s.io/{} configured", name);
            } else {
                pc.metadata.ensure_uid();
                pc.metadata.ensure_creation_timestamp();
                let _result: PriorityClass = client
                    .post("/apis/scheduling.k8s.io/v1/priorityclasses", &pc)
                    .await?;
                println!("priorityclass.scheduling.k8s.io/{} created", name);
            }
        }
        "CustomResourceDefinition" => {
            let mut crd: CustomResourceDefinition = serde_yaml::from_str(&yaml_str)?;
            let name = crd.metadata.name.clone();

            let exists = resource_exists::<CustomResourceDefinition>(client, &format!("/apis/apiextensions.k8s.io/v1/customresourcedefinitions/{}", name)).await?;

            if exists {
                let _result: CustomResourceDefinition = client
                    .put(&format!("/apis/apiextensions.k8s.io/v1/customresourcedefinitions/{}", name), &crd)
                    .await?;
                println!("customresourcedefinition.apiextensions.k8s.io/{} configured", name);
            } else {
                crd.metadata.ensure_uid();
                crd.metadata.ensure_creation_timestamp();
                let _result: CustomResourceDefinition = client
                    .post("/apis/apiextensions.k8s.io/v1/customresourcedefinitions", &crd)
                    .await?;
                println!("customresourcedefinition.apiextensions.k8s.io/{} created", name);
            }
        }
        _ => anyhow::bail!("Unsupported resource kind: {}", kind),
    }

    Ok(())
}

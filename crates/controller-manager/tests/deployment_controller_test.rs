// Integration tests for Deployment Controller
// Note: These tests use in-memory storage for fast, isolated testing

use rusternetes_common::resources::pod::*;
use rusternetes_common::resources::*;
use rusternetes_common::types::{ObjectMeta, Phase, TypeMeta, LabelSelector};
use rusternetes_controller_manager::controllers::deployment::DeploymentController;
use rusternetes_storage::{build_key, memory::MemoryStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn setup_test() -> Arc<MemoryStorage> {
    let storage = Arc::new(MemoryStorage::new());

    // Clean up test data (memory storage starts fresh)
    storage.clear();

    storage
}

fn create_test_deployment(name: &str, namespace: &str, replicas: i32) -> Deployment {
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), name.to_string());

    Deployment {
        type_meta: TypeMeta {
            kind: "Deployment".to_string(),
            api_version: "apps/v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new(name);
            meta.namespace = Some(namespace.to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: DeploymentSpec {
            replicas,
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                match_expressions: None,
            },
            template: PodTemplateSpec {
                metadata: Some({
                    let mut meta = ObjectMeta::new(&format!("{}-pod", name));
                    meta.labels = Some(labels);
                    meta
                }),
                spec: PodSpec {
                    containers: vec![Container {
                        name: "nginx".to_string(),
                        image: "nginx:1.25-alpine".to_string(),
                        image_pull_policy: Some("IfNotPresent".to_string()),
                        ports: Some(vec![]),
                        env: None,
                        volume_mounts: None,
                        liveness_probe: None,
                        readiness_probe: None,
                        startup_probe: None,
                resources: None,                        working_dir: None,
                        command: None,
                        args: None,
                        security_context: None,
                    }],
                    init_containers: None,
                    restart_policy: Some("Always".to_string()),
                    node_selector: None,
                    node_name: None,
                    volumes: None,
                    affinity: None,
                    tolerations: None,
                    service_account_name: None,
                    priority: None,
                    priority_class_name: None,
                hostname: None,
                host_network: None,
                    host_pid: None,
                    host_ipc: None,
                },
            },
            strategy: None,
            min_ready_seconds: None,
            revision_history_limit: None,
        },
        status: Some(DeploymentStatus {
            replicas: Some(0),
            ready_replicas: Some(0),
            available_replicas: Some(0),
            unavailable_replicas: Some(0),
            updated_replicas: Some(0),
        }),
    }
}

#[tokio::test]
async fn test_deployment_creates_pods() {
    let storage = setup_test().await;

    // Create deployment with replicas: 3
    let deployment = create_test_deployment("nginx", "default", 3);
    let key = build_key("deployments", Some("default"), "nginx");
    storage.create(&key, &deployment).await.unwrap();

    // Run controller
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 3 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should create 3 pods");

    // Verify pods have correct labels
    for pod in &pods {
        let labels = pod.metadata.labels.as_ref().expect("Pod should have labels");
        assert_eq!(labels.get("app"), Some(&"nginx".to_string()));
    }

    // Verify all pods are in Pending phase initially
    for pod in &pods {
        assert_eq!(pod.status.as_ref().unwrap().phase, Phase::Pending);
    }
}

#[tokio::test]
async fn test_deployment_scales_up() {
    let storage = setup_test().await;

    // Create deployment with 2 replicas
    let mut deployment = create_test_deployment("app", "default", 2);
    let key = build_key("deployments", Some("default"), "app");
    storage.create(&key, &deployment).await.unwrap();

    // Run controller to create initial pods
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 2 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should initially create 2 pods");

    // Update deployment to 5 replicas
    deployment.spec.replicas = 5;
    storage.update(&key, &deployment).await.unwrap();

    // Run controller again
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 5 pods now exist
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 5, "Should scale up to 5 pods");
}

#[tokio::test]
async fn test_deployment_scales_down() {
    let storage = setup_test().await;

    // Create deployment with 5 replicas
    let mut deployment = create_test_deployment("app", "default", 5);
    let key = build_key("deployments", Some("default"), "app");
    storage.create(&key, &deployment).await.unwrap();

    // Run controller to create initial pods
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 5 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 5, "Should initially create 5 pods");

    // Update deployment to 2 replicas
    deployment.spec.replicas = 2;
    storage.update(&key, &deployment).await.unwrap();

    // Run controller again
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 2 pods remain
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should scale down to 2 pods");
}

#[tokio::test]
async fn test_deployment_self_healing() {
    let storage = setup_test().await;

    // Create deployment with 3 replicas
    let deployment = create_test_deployment("app", "default", 3);
    let key = build_key("deployments", Some("default"), "app");
    storage.create(&key, &deployment).await.unwrap();

    // Run controller to create initial pods
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 3 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should create 3 pods");

    // Delete one pod to simulate failure
    let pod_to_delete = &pods[0];
    let pod_key = build_key("pods", Some("default"), &pod_to_delete.metadata.name);
    storage.delete(&pod_key).await.unwrap();

    // Verify only 2 pods remain
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should have 2 pods after deletion");

    // Run controller again to heal
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 3 pods exist again (self-healed)
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should self-heal back to 3 pods");
}

#[tokio::test]
async fn test_deployment_selector_matching() {
    let storage = setup_test().await;

    // Create deployment with specific selector
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "myapp".to_string());
    labels.insert("tier".to_string(), "frontend".to_string());

    let deployment = Deployment {
        type_meta: TypeMeta {
            kind: "Deployment".to_string(),
            api_version: "apps/v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("frontend");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: DeploymentSpec {
            replicas: 2,
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                match_expressions: None,
            },
            template: PodTemplateSpec {
                metadata: Some({
                    let mut meta = ObjectMeta::new("frontend-pod");
                    meta.labels = Some(labels);
                    meta
                }),
                spec: PodSpec {
                    containers: vec![Container {
                        name: "nginx".to_string(),
                        image: "nginx:1.25-alpine".to_string(),
                        image_pull_policy: Some("IfNotPresent".to_string()),
                        ports: Some(vec![]),
                        env: None,
                        volume_mounts: None,
                        liveness_probe: None,
                        readiness_probe: None,
                        startup_probe: None,
                resources: None,                        working_dir: None,
                        command: None,
                        args: None,
                        security_context: None,
                    }],
                    init_containers: None,
                    restart_policy: Some("Always".to_string()),
                    node_selector: None,
                    node_name: None,
                    volumes: None,
                    affinity: None,
                    tolerations: None,
                    service_account_name: None,
                    priority: None,
                    priority_class_name: None,
                hostname: None,
                host_network: None,
                    host_pid: None,
                    host_ipc: None,
                },
            },
            strategy: None,
            min_ready_seconds: None,
            revision_history_limit: None,
        },
        status: Some(DeploymentStatus {
            replicas: Some(0),
            ready_replicas: Some(0),
            available_replicas: Some(0),
            unavailable_replicas: Some(0),
            updated_replicas: Some(0),
        }),
    };

    let key = build_key("deployments", Some("default"), "frontend");
    storage.create(&key, &deployment).await.unwrap();

    // Create a pod that doesn't match the selector (missing "tier" label)
    let mut non_matching_labels = HashMap::new();
    non_matching_labels.insert("app".to_string(), "myapp".to_string());

    let non_matching_pod = Pod {
        type_meta: TypeMeta {
            kind: "Pod".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("non-matching-pod");
            meta.namespace = Some("default".to_string());
            meta.labels = Some(non_matching_labels);
            meta
        },
        spec: Some(PodSpec {
            containers: vec![Container {
                name: "nginx".to_string(),
                image: "nginx:1.25-alpine".to_string(),
                image_pull_policy: Some("IfNotPresent".to_string()),
                ports: Some(vec![]),
                env: None,
                volume_mounts: None,
                liveness_probe: None,
                readiness_probe: None,
                startup_probe: None,
                resources: None,
                        working_dir: None,
                command: None,
                args: None,
                        security_context: None,
            }],
            init_containers: None,
            restart_policy: Some("Always".to_string()),
            node_selector: None,
            node_name: None,
            volumes: None,
            affinity: None,
            tolerations: None,
            service_account_name: None,
            priority: None,
            priority_class_name: None,
                hostname: None,
                host_network: None,
                    host_pid: None,
                    host_ipc: None,
        }),
        status: Some(PodStatus {
            phase: Phase::Pending,
            message: None,
            reason: None,
            host_ip: None,
            pod_ip: None,
            container_statuses: None,
            init_container_statuses: None,
        }),
    };

    let pod_key = build_key("pods", Some("default"), "non-matching-pod");
    storage.create(&pod_key, &non_matching_pod).await.unwrap();

    // Run controller
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 2 pods created (deployment should ignore the non-matching pod)
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    // Total should be 3: 1 non-matching + 2 from deployment
    assert_eq!(pods.len(), 3, "Should have 1 non-matching pod + 2 deployment pods");

    // Count pods with matching labels
    let matching_pods = pods.iter().filter(|p| {
        if let Some(labels) = &p.metadata.labels {
            labels.get("app") == Some(&"myapp".to_string()) &&
            labels.get("tier") == Some(&"frontend".to_string())
        } else {
            false
        }
    }).count();

    assert_eq!(matching_pods, 2, "Should have exactly 2 pods matching deployment selector");
}

#[tokio::test]
async fn test_deployment_zero_replicas() {
    let storage = setup_test().await;

    // Create deployment with 0 replicas
    let deployment = create_test_deployment("app", "default", 0);
    let key = build_key("deployments", Some("default"), "app");
    storage.create(&key, &deployment).await.unwrap();

    // Run controller
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify no pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 0, "Should not create any pods for 0 replicas");
}

#[tokio::test]
async fn test_deployment_multiple_namespaces() {
    let storage = setup_test().await;

    // Create deployment in namespace1
    let deployment1 = create_test_deployment("app", "namespace1", 2);
    let key1 = build_key("deployments", Some("namespace1"), "app");
    storage.create(&key1, &deployment1).await.unwrap();

    // Create deployment in namespace2
    let deployment2 = create_test_deployment("app", "namespace2", 3);
    let key2 = build_key("deployments", Some("namespace2"), "app");
    storage.create(&key2, &deployment2).await.unwrap();

    // Run controller
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify pods in namespace1
    let pods1: Vec<Pod> = storage.list("/registry/pods/namespace1/").await.unwrap();
    assert_eq!(pods1.len(), 2, "Should create 2 pods in namespace1");

    // Verify pods in namespace2
    let pods2: Vec<Pod> = storage.list("/registry/pods/namespace2/").await.unwrap();
    assert_eq!(pods2.len(), 3, "Should create 3 pods in namespace2");
}

#[tokio::test]
async fn test_deployment_preserves_existing_matching_pods() {
    let storage = setup_test().await;

    // Create deployment with 3 replicas
    let deployment = create_test_deployment("app", "default", 3);

    // Manually create 2 matching pods first
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "app".to_string());

    for i in 0..2 {
        let pod = Pod {
            type_meta: TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: {
                let mut meta = ObjectMeta::new(&format!("existing-pod-{}", i));
                meta.namespace = Some("default".to_string());
                meta.labels = Some(labels.clone());
                meta
            },
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "nginx".to_string(),
                    image: "nginx:1.25-alpine".to_string(),
                    image_pull_policy: Some("IfNotPresent".to_string()),
                    ports: Some(vec![]),
                    env: None,
                    volume_mounts: None,
                    liveness_probe: None,
                    readiness_probe: None,
                    startup_probe: None,
                resources: None,                    working_dir: None,
                    command: None,
                    args: None,
                        security_context: None,
                }],
                init_containers: None,
                restart_policy: Some("Always".to_string()),
                node_selector: None,
                node_name: None,
                volumes: None,
                affinity: None,
                tolerations: None,
                service_account_name: None,
                priority: None,
                priority_class_name: None,
                hostname: None,
                host_network: None,
                    host_pid: None,
                    host_ipc: None,
            }),
            status: Some(PodStatus {
                phase: Phase::Running,
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                container_statuses: None,
                init_container_statuses: None,
            }),
        };

        let pod_key = build_key("pods", Some("default"), &format!("existing-pod-{}", i));
        storage.create(&pod_key, &pod).await.unwrap();
    }

    // Now create the deployment
    let key = build_key("deployments", Some("default"), "app");
    storage.create(&key, &deployment).await.unwrap();

    // Run controller
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify total is 3 pods (2 existing + 1 new)
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should have 3 total pods (2 existing + 1 created)");

    // Verify the existing pods are still there
    let existing_pod_names: Vec<&str> = pods
        .iter()
        .filter(|p| p.metadata.name.starts_with("existing-pod"))
        .map(|p| p.metadata.name.as_str())
        .collect();

    assert_eq!(existing_pod_names.len(), 2, "Both existing pods should be preserved");
}

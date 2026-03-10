// Integration tests for StatefulSet Controller

use rusternetes_common::resources::pod::*;
use rusternetes_common::resources::*;
use rusternetes_common::types::{ObjectMeta, Phase, TypeMeta, LabelSelector};
use rusternetes_controller_manager::controllers::statefulset::StatefulSetController;
use rusternetes_storage::{build_key, Storage};
use rusternetes_storage::etcd::EtcdStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn setup_test() -> Arc<EtcdStorage> {
    let endpoints = vec!["http://localhost:2379".to_string()];
    Arc::new(EtcdStorage::new(endpoints).await.expect("Failed to create EtcdStorage"))
}

fn create_test_statefulset(name: &str, namespace: &str, replicas: i32) -> StatefulSet {
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), name.to_string());

    StatefulSet {
        type_meta: TypeMeta {
            kind: "StatefulSet".to_string(),
            api_version: "apps/v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new(name);
            meta.namespace = Some(namespace.to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: StatefulSetSpec {
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
                },
            },
            service_name: format!("{}-headless", name),
            pod_management_policy: Some("OrderedReady".to_string()),
            update_strategy: None,
        },
        status: Some(StatefulSetStatus {
            replicas: 0,
            ready_replicas: 0,
            current_replicas: 0,
            updated_replicas: 0,
        }),
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_statefulset_creates_ordered_pods() {
    let storage = setup_test().await;

    // Clean up any existing test data
    let _ = storage.delete("/registry/statefulsets/test-ns/web").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create statefulset with 3 replicas
    let statefulset = create_test_statefulset("web", "test-ns", 3);
    let key = build_key("statefulsets", Some("test-ns"), "web");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Verify 3 pods created with ordered names
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should create 3 pods");

    // Verify pod names are ordered: web-0, web-1, web-2
    let mut pod_names: Vec<String> = pods.iter().map(|p| p.metadata.name.clone()).collect();
    pod_names.sort();
    assert_eq!(pod_names, vec!["web-0", "web-1", "web-2"]);

    // Cleanup
    let _ = storage.delete(&key).await;
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_statefulset_scales_up_ordered() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/statefulsets/test-ns/web").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create statefulset with 2 replicas
    let mut statefulset = create_test_statefulset("web", "test-ns", 2);
    let key = build_key("statefulsets", Some("test-ns"), "web");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Verify 2 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should create 2 pods initially");

    // Scale up to 4 replicas
    statefulset.spec.replicas = 4;
    storage.update(&key, &statefulset).await.unwrap();

    // Run controller again
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Verify 4 pods exist
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 4, "Should scale up to 4 pods");

    // Verify new pods are web-2 and web-3
    let mut pod_names: Vec<String> = pods.iter().map(|p| p.metadata.name.clone()).collect();
    pod_names.sort();
    assert_eq!(pod_names, vec!["web-0", "web-1", "web-2", "web-3"]);

    // Cleanup
    let _ = storage.delete(&key).await;
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_statefulset_scales_down_reverse_order() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/statefulsets/test-ns/web").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create statefulset with 4 replicas
    let mut statefulset = create_test_statefulset("web", "test-ns", 4);
    let key = build_key("statefulsets", Some("test-ns"), "web");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Verify 4 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 4);

    // Scale down to 2 replicas
    statefulset.spec.replicas = 2;
    storage.update(&key, &statefulset).await.unwrap();

    // Run controller again
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Verify only 2 pods remain
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should scale down to 2 pods");

    // Verify remaining pods are web-0 and web-1 (highest ordinals deleted first)
    let mut pod_names: Vec<String> = pods.iter().map(|p| p.metadata.name.clone()).collect();
    pod_names.sort();
    assert_eq!(pod_names, vec!["web-0", "web-1"]);

    // Cleanup
    let _ = storage.delete(&key).await;
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_statefulset_stable_pod_names() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/statefulsets/test-ns/db").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create statefulset
    let statefulset = create_test_statefulset("db", "test-ns", 3);
    let key = build_key("statefulsets", Some("test-ns"), "db");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Get initial pods
    let initial_pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    let initial_names: Vec<String> = initial_pods.iter().map(|p| p.metadata.name.clone()).collect();

    // Delete one pod (simulate pod failure)
    let _ = storage.delete(&build_key("pods", Some("test-ns"), "db-1")).await;

    // Run controller to recreate
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Verify pod names remain stable
    let final_pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    let mut final_names: Vec<String> = final_pods.iter().map(|p| p.metadata.name.clone()).collect();
    final_names.sort();

    assert_eq!(final_names, initial_names, "Pod names should remain stable");

    // Cleanup
    let _ = storage.delete(&key).await;
    for pod in final_pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

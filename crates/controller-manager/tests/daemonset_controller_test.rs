// Integration tests for DaemonSet Controller

use rusternetes_common::resources::pod::*;
use rusternetes_common::resources::*;
use rusternetes_common::types::{ObjectMeta, Phase, TypeMeta, LabelSelector};
use rusternetes_controller_manager::controllers::daemonset::DaemonSetController;
use rusternetes_storage::{build_key, Storage};
use rusternetes_storage::etcd::EtcdStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn setup_test() -> Arc<EtcdStorage> {
    let endpoints = vec!["http://localhost:2379".to_string()];
    Arc::new(EtcdStorage::new(endpoints).await.expect("Failed to create EtcdStorage"))
}

fn create_test_node(name: &str, labels: Option<HashMap<String, String>>) -> Node {
    Node {
        type_meta: TypeMeta {
            kind: "Node".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: ObjectMeta {
            name: name.to_string(),
            namespace: None,
            labels,
            annotations: None,
            uid: uuid::Uuid::new_v4().to_string(),
            creation_timestamp: None,
            deletion_timestamp: None,
            resource_version: None,
            deletion_grace_period_seconds: None,
            finalizers: None,
            owner_references: None,
        },
        spec: Some(NodeSpec {
            pod_cidr: None,
            provider_id: None,
            unschedulable: None,
            taints: None,
        }),
        status: None,
    }
}

fn create_test_daemonset(name: &str, namespace: &str, node_selector: Option<HashMap<String, String>>) -> DaemonSet {
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), name.to_string());

    DaemonSet {
        type_meta: TypeMeta {
            kind: "DaemonSet".to_string(),
            api_version: "apps/v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new(name);
            meta.namespace = Some(namespace.to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: DaemonSetSpec {
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
                        name: "logger".to_string(),
                        image: "busybox:latest".to_string(),
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
                    node_selector,
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
            update_strategy: None,
        },
        status: Some(DaemonSetStatus {
            desired_number_scheduled: 0,
            current_number_scheduled: 0,
            number_ready: 0,
            number_misscheduled: 0,
        }),
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_daemonset_creates_pod_per_node() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/daemonsets/test-ns/logger").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create 3 nodes
    for i in 0..3 {
        let node = create_test_node(&format!("node-{}", i), None);
        let key = build_key("nodes", None, &format!("node-{}", i));
        storage.create(&key, &node).await.unwrap();
    }

    // Create DaemonSet (no node selector = all nodes)
    let daemonset = create_test_daemonset("logger", "test-ns", None);
    let key = build_key("daemonsets", Some("test-ns"), "logger");
    storage.create(&key, &daemonset).await.unwrap();

    // Run controller
    let controller = DaemonSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 3 pods created (one per node)
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should create one pod per node");

    // Cleanup
    let _ = storage.delete(&key).await;
    for i in 0..3 {
        let _ = storage.delete(&build_key("nodes", None, &format!("node-{}", i))).await;
    }
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_daemonset_node_selector() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/daemonsets/test-ns/ssd-logger").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create nodes with different labels
    let mut ssd_labels = HashMap::new();
    ssd_labels.insert("disktype".to_string(), "ssd".to_string());

    for i in 0..2 {
        let node = create_test_node(&format!("ssd-node-{}", i), Some(ssd_labels.clone()));
        let key = build_key("nodes", None, &format!("ssd-node-{}", i));
        storage.create(&key, &node).await.unwrap();
    }

    // Create nodes without SSD label
    for i in 0..2 {
        let node = create_test_node(&format!("hdd-node-{}", i), None);
        let key = build_key("nodes", None, &format!("hdd-node-{}", i));
        storage.create(&key, &node).await.unwrap();
    }

    // Create DaemonSet with node selector for disktype=ssd
    let mut node_selector = HashMap::new();
    node_selector.insert("disktype".to_string(), "ssd".to_string());
    let daemonset = create_test_daemonset("ssd-logger", "test-ns", Some(node_selector));
    let key = build_key("daemonsets", Some("test-ns"), "ssd-logger");
    storage.create(&key, &daemonset).await.unwrap();

    // Run controller
    let controller = DaemonSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify only 2 pods created (on SSD nodes only)
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should only create pods on nodes matching selector");

    // Cleanup
    let _ = storage.delete(&key).await;
    for i in 0..2 {
        let _ = storage.delete(&build_key("nodes", None, &format!("ssd-node-{}", i))).await;
        let _ = storage.delete(&build_key("nodes", None, &format!("hdd-node-{}", i))).await;
    }
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_daemonset_node_addition() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/daemonsets/test-ns/logger").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create 2 nodes initially
    for i in 0..2 {
        let node = create_test_node(&format!("node-{}", i), None);
        let key = build_key("nodes", None, &format!("node-{}", i));
        storage.create(&key, &node).await.unwrap();
    }

    // Create DaemonSet
    let daemonset = create_test_daemonset("logger", "test-ns", None);
    let key = build_key("daemonsets", Some("test-ns"), "logger");
    storage.create(&key, &daemonset).await.unwrap();

    // Run controller
    let controller = DaemonSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 2 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 2);

    // Add a new node
    let node = create_test_node("node-2", None);
    let node_key = build_key("nodes", None, "node-2");
    storage.create(&node_key, &node).await.unwrap();

    // Run controller again
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 3 pods now exist
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should create pod on newly added node");

    // Cleanup
    let _ = storage.delete(&key).await;
    for i in 0..3 {
        let _ = storage.delete(&build_key("nodes", None, &format!("node-{}", i))).await;
    }
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

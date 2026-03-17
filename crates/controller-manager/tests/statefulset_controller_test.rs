// StatefulSet Controller Integration Tests
// Tests the StatefulSet controller's ability to manage stateful workloads with stable identities

use rusternetes_common::resources::pod::*;
use rusternetes_common::resources::*;
use rusternetes_common::types::{LabelSelector, ObjectMeta, Phase, TypeMeta};
use rusternetes_controller_manager::controllers::statefulset::StatefulSetController;
use rusternetes_storage::{build_key, memory::MemoryStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;

async fn setup_test() -> Arc<MemoryStorage> {
    let storage = Arc::new(MemoryStorage::new());
    storage.clear();
    storage
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
            replicas: Some(replicas),
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
                        ports: None,
                        env: None,
                        volume_mounts: None,
                        liveness_probe: None,
                        readiness_probe: None,
                        startup_probe: None,
                        resources: None,
                        working_dir: None,
                        command: None,
                        args: None,
                        restart_policy: None,
                        resize_policy: None,
                        security_context: None,
                        lifecycle: None,
                        termination_message_path: None,
                        termination_message_policy: None,
                        stdin: None,
                        stdin_once: None,
                        tty: None,
                        env_from: None,
                        volume_devices: None,
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
                    subdomain: None,
                    host_network: None,
                    host_pid: None,
                    host_ipc: None,
                    automount_service_account_token: None,
                    ephemeral_containers: None,
                    overhead: None,
                    scheduler_name: None,
                    topology_spread_constraints: None,
                    resource_claims: None,
                    active_deadline_seconds: None,
                    dns_policy: None,
                    dns_config: None,
                    security_context: None,
                    image_pull_secrets: None,
                    share_process_namespace: None,
                    readiness_gates: None,
                    runtime_class_name: None,
                    enable_service_links: None,
                    preemption_policy: None,
                    host_users: None,
                    set_hostname_as_fqdn: None,
                    termination_grace_period_seconds: None,
                    host_aliases: None,
                    os: None,
                    scheduling_gates: None,
                    resources: None,
                },
            },
            service_name: format!("{}-headless", name),
            pod_management_policy: Some("OrderedReady".to_string()),
            update_strategy: None,
        min_ready_seconds: None,
        revision_history_limit: None,
        volume_claim_templates: None,
        persistent_volume_claim_retention_policy: None,
        },
        status: Some(StatefulSetStatus {
            replicas: 0,
            ready_replicas: Some(0),
            current_replicas: Some(0),
            updated_replicas: Some(0),
        available_replicas: None,
        collision_count: None,
        observed_generation: None,
        current_revision: None,
        update_revision: None,
        conditions: None,
        }),
    }
}

#[tokio::test]
async fn test_statefulset_creates_ordered_pods() {
    let storage = setup_test().await;

    // Create statefulset with 3 replicas
    let statefulset = create_test_statefulset("web", "default", 3);
    let key = build_key("statefulsets", Some("default"), "web");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();

    // Verify 3 pods created with ordered names
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 3, "Should create 3 pods");

    // Verify pod names are ordered: web-0, web-1, web-2
    let mut pod_names: Vec<String> = pods.iter().map(|p| p.metadata.name.clone()).collect();
    pod_names.sort();
    assert_eq!(pod_names, vec!["web-0", "web-1", "web-2"]);

    // Verify each pod has the stateful identity label
    for pod in &pods {
        let labels = pod
            .metadata
            .labels
            .as_ref()
            .expect("Pod should have labels");
        assert!(labels.contains_key("statefulset.kubernetes.io/pod-name"));
    }
}

#[tokio::test]
async fn test_statefulset_scales_up_ordered() {
    let storage = setup_test().await;

    // Create statefulset with 2 replicas
    let mut statefulset = create_test_statefulset("web", "default", 2);
    let key = build_key("statefulsets", Some("default"), "web");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();

    // Verify 2 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should create 2 pods initially");

    // Scale up to 4 replicas
    statefulset.spec.replicas = Some(4);
    storage.update(&key, &statefulset).await.unwrap();

    // Run controller again
    controller.reconcile_all().await.unwrap();

    // Verify 4 pods exist
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 4, "Should scale up to 4 pods");

    // Verify new pods are web-2 and web-3
    let mut pod_names: Vec<String> = pods.iter().map(|p| p.metadata.name.clone()).collect();
    pod_names.sort();
    assert_eq!(pod_names, vec!["web-0", "web-1", "web-2", "web-3"]);
}

#[tokio::test]
async fn test_statefulset_scales_down_reverse_order() {
    let storage = setup_test().await;

    // Create statefulset with 4 replicas
    let mut statefulset = create_test_statefulset("web", "default", 4);
    let key = build_key("statefulsets", Some("default"), "web");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();

    // Verify 4 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 4);

    // Scale down to 2 replicas
    statefulset.spec.replicas = Some(2);
    storage.update(&key, &statefulset).await.unwrap();

    // Run controller again
    controller.reconcile_all().await.unwrap();

    // Verify only 2 pods remain
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should scale down to 2 pods");

    // Verify remaining pods are web-0 and web-1 (highest ordinals deleted first)
    let mut pod_names: Vec<String> = pods.iter().map(|p| p.metadata.name.clone()).collect();
    pod_names.sort();
    assert_eq!(pod_names, vec!["web-0", "web-1"]);
}

#[tokio::test]
async fn test_statefulset_updates_status() {
    let storage = setup_test().await;

    // Create statefulset
    let statefulset = create_test_statefulset("status-test", "default", 3);
    let key = build_key("statefulsets", Some("default"), "status-test");
    storage.create(&key, &statefulset).await.unwrap();

    // Run controller
    let controller = StatefulSetController::new(storage.clone());
    controller.reconcile_all().await.unwrap();

    // Verify status was updated
    let updated_ss: StatefulSet = storage.get(&key).await.unwrap();
    let status = updated_ss.status.expect("Status should be set");

    assert_eq!(status.replicas, 3, "Status replicas should match actual");
    assert_eq!(status.current_replicas, Some(3), "Current replicas should be 3");
}

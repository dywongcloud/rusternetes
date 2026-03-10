//! Integration tests for StatefulSet handler
//!
//! Tests all CRUD operations, edge cases, and error handling for statefulsets

use rusternetes_common::resources::{StatefulSet, StatefulSetSpec, StatefulSetStatus, LabelSelector, PodTemplateSpec, PodSpec, PersistentVolumeClaim, PersistentVolumeClaimSpec, VolumeResourceRequirements};
use rusternetes_common::types::{Container, Metadata};
use rusternetes_storage::{build_key, build_prefix, memory::MemoryStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;

/// Helper function to create a test statefulset
fn create_test_statefulset(name: &str, namespace: &str, replicas: i32) -> StatefulSet {
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), name.to_string());

    StatefulSet {
        metadata: Metadata {
            name: name.to_string(),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            uid: None,
            creation_timestamp: None,
            resource_version: None,
            finalizers: None,
            deletion_timestamp: None,
            owner_references: None,
            annotations: None,
            generation: None,
        },
        spec: StatefulSetSpec {
            replicas: Some(replicas),
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                match_expressions: None,
            },
            template: PodTemplateSpec {
                metadata: Some(Metadata {
                    name: format!("{}-pod", name),
                    namespace: Some(namespace.to_string()),
                    labels: Some(labels),
                    uid: None,
                    creation_timestamp: None,
                    resource_version: None,
                    finalizers: None,
                    deletion_timestamp: None,
                    owner_references: None,
                    annotations: None,
                    generation: None,
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "app".to_string(),
                        image: "nginx:latest".to_string(),
                        command: None,
                        args: None,
                        env: None,
                        ports: None,
                        volume_mounts: None,
                        resources: None,
                        security_context: None,
                        liveness_probe: None,
                        readiness_probe: None,
                        startup_probe: None,
                        lifecycle: None,
                        image_pull_policy: None,
                        stdin: None,
                        stdin_once: None,
                        tty: None,
                        working_dir: None,
                        termination_message_path: None,
                        termination_message_policy: None,
                    }],
                    init_containers: None,
                    restart_policy: Some("Always".to_string()),
                    termination_grace_period_seconds: Some(30),
                    dns_policy: Some("ClusterFirst".to_string()),
                    service_account_name: None,
                    automount_service_account_token: None,
                    node_selector: None,
                    node_name: None,
                    affinity: None,
                    tolerations: None,
                    host_network: None,
                    host_pid: None,
                    host_ipc: None,
                    hostname: None,
                    subdomain: None,
                    priority_class_name: None,
                    priority: None,
                    scheduler_name: None,
                    volumes: None,
                    image_pull_secrets: None,
                    security_context: None,
                    runtime_class_name: None,
                    enable_service_links: None,
                    preemption_policy: None,
                    overhead: None,
                    topology_spread_constraints: None,
                    set_hostname_as_fqdn: None,
                    os: None,
                    scheduling_gates: None,
                    resource_claims: None,
                }),
            },
            service_name: format!("{}-service", name),
            pod_management_policy: Some("OrderedReady".to_string()),
            update_strategy: None,
            revision_history_limit: Some(10),
            min_ready_seconds: Some(0),
            volume_claim_templates: None,
            persistent_volume_claim_retention_policy: None,
            ordinals: None,
        },
        status: Some(StatefulSetStatus {
            replicas: 0,
            ready_replicas: Some(0),
            current_replicas: Some(0),
            updated_replicas: Some(0),
            available_replicas: Some(0),
            observed_generation: Some(0),
            collision_count: None,
            conditions: None,
            current_revision: None,
            update_revision: None,
        }),
    }
}

#[tokio::test]
async fn test_statefulset_create_and_get() {
    let storage = Arc::new(MemoryStorage::new());

    let statefulset = create_test_statefulset("test-sts", "default", 3);
    let key = build_key("statefulsets", Some("default"), "test-sts");

    // Create
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.metadata.name, "test-sts");
    assert_eq!(created.metadata.namespace, Some("default".to_string()));
    assert_eq!(created.spec.replicas, Some(3));
    assert_eq!(created.spec.service_name, "test-sts-service");
    assert!(created.metadata.uid.is_some());

    // Get
    let retrieved: StatefulSet = storage.get(&key).await.unwrap();
    assert_eq!(retrieved.metadata.name, "test-sts");
    assert_eq!(retrieved.spec.replicas, Some(3));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_update() {
    let storage = Arc::new(MemoryStorage::new());

    let mut statefulset = create_test_statefulset("test-update-sts", "default", 3);
    let key = build_key("statefulsets", Some("default"), "test-update-sts");

    // Create
    storage.create(&key, &statefulset).await.unwrap();

    // Update replicas
    statefulset.spec.replicas = Some(5);
    let updated: StatefulSet = storage.update(&key, &statefulset).await.unwrap();
    assert_eq!(updated.spec.replicas, Some(5));

    // Verify update
    let retrieved: StatefulSet = storage.get(&key).await.unwrap();
    assert_eq!(retrieved.spec.replicas, Some(5));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_delete() {
    let storage = Arc::new(MemoryStorage::new());

    let statefulset = create_test_statefulset("test-delete-sts", "default", 3);
    let key = build_key("statefulsets", Some("default"), "test-delete-sts");

    // Create
    storage.create(&key, &statefulset).await.unwrap();

    // Delete
    storage.delete(&key).await.unwrap();

    // Verify deletion
    let result = storage.get::<StatefulSet>(&key).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_statefulset_list() {
    let storage = Arc::new(MemoryStorage::new());

    // Create multiple statefulsets
    let sts1 = create_test_statefulset("sts-1", "default", 3);
    let sts2 = create_test_statefulset("sts-2", "default", 5);
    let sts3 = create_test_statefulset("sts-3", "default", 2);

    let key1 = build_key("statefulsets", Some("default"), "sts-1");
    let key2 = build_key("statefulsets", Some("default"), "sts-2");
    let key3 = build_key("statefulsets", Some("default"), "sts-3");

    storage.create(&key1, &sts1).await.unwrap();
    storage.create(&key2, &sts2).await.unwrap();
    storage.create(&key3, &sts3).await.unwrap();

    // List
    let prefix = build_prefix("statefulsets", Some("default"));
    let statefulsets: Vec<StatefulSet> = storage.list(&prefix).await.unwrap();

    assert!(statefulsets.len() >= 3);
    let names: Vec<String> = statefulsets.iter().map(|sts| sts.metadata.name.clone()).collect();
    assert!(names.contains(&"sts-1".to_string()));
    assert!(names.contains(&"sts-2".to_string()));
    assert!(names.contains(&"sts-3".to_string()));

    // Clean up
    storage.delete(&key1).await.unwrap();
    storage.delete(&key2).await.unwrap();
    storage.delete(&key3).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_list_across_namespaces() {
    let storage = Arc::new(MemoryStorage::new());

    // Create statefulsets in different namespaces
    let sts1 = create_test_statefulset("sts-ns1", "namespace-1", 3);
    let sts2 = create_test_statefulset("sts-ns2", "namespace-2", 5);

    let key1 = build_key("statefulsets", Some("namespace-1"), "sts-ns1");
    let key2 = build_key("statefulsets", Some("namespace-2"), "sts-ns2");

    storage.create(&key1, &sts1).await.unwrap();
    storage.create(&key2, &sts2).await.unwrap();

    // List all (no namespace filter)
    let prefix = build_prefix("statefulsets", None);
    let statefulsets: Vec<StatefulSet> = storage.list(&prefix).await.unwrap();

    // Should find at least our 2 statefulsets
    assert!(statefulsets.len() >= 2);

    // Clean up
    storage.delete(&key1).await.unwrap();
    storage.delete(&key2).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_with_status() {
    let storage = Arc::new(MemoryStorage::new());

    let mut statefulset = create_test_statefulset("test-status", "default", 3);
    statefulset.status = Some(StatefulSetStatus {
        replicas: 3,
        ready_replicas: Some(2),
        current_replicas: Some(3),
        updated_replicas: Some(3),
        available_replicas: Some(2),
        observed_generation: Some(1),
        collision_count: None,
        conditions: None,
        current_revision: Some("rev-1".to_string()),
        update_revision: Some("rev-2".to_string()),
    });

    let key = build_key("statefulsets", Some("default"), "test-status");

    // Create with status
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.status.as_ref().unwrap().replicas, 3);
    assert_eq!(created.status.as_ref().unwrap().ready_replicas, Some(2));
    assert_eq!(created.status.as_ref().unwrap().current_revision, Some("rev-1".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_ordered_ready_policy() {
    let storage = Arc::new(MemoryStorage::new());

    let statefulset = create_test_statefulset("test-ordered", "default", 3);
    let key = build_key("statefulsets", Some("default"), "test-ordered");

    // Create with OrderedReady policy
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.spec.pod_management_policy, Some("OrderedReady".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_parallel_policy() {
    let storage = Arc::new(MemoryStorage::new());

    let mut statefulset = create_test_statefulset("test-parallel", "default", 3);
    statefulset.spec.pod_management_policy = Some("Parallel".to_string());

    let key = build_key("statefulsets", Some("default"), "test-parallel");

    // Create with Parallel policy
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.spec.pod_management_policy, Some("Parallel".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_with_volume_claim_templates() {
    let storage = Arc::new(MemoryStorage::new());

    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "10Gi".to_string());

    let pvc_template = PersistentVolumeClaim {
        metadata: Metadata {
            name: "data".to_string(),
            namespace: None,
            labels: None,
            uid: None,
            creation_timestamp: None,
            resource_version: None,
            finalizers: None,
            deletion_timestamp: None,
            owner_references: None,
            annotations: None,
            generation: None,
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: Some(vec!["ReadWriteOnce".to_string()]),
            resources: Some(VolumeResourceRequirements {
                requests: Some(requests),
                limits: None,
            }),
            storage_class_name: Some("standard".to_string()),
            volume_mode: None,
            selector: None,
            volume_name: None,
            data_source: None,
            data_source_ref: None,
        },
        status: None,
    };

    let mut statefulset = create_test_statefulset("test-pvc", "default", 3);
    statefulset.spec.volume_claim_templates = Some(vec![pvc_template]);

    let key = build_key("statefulsets", Some("default"), "test-pvc");

    // Create with volume claim templates
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert!(created.spec.volume_claim_templates.is_some());
    let templates = created.spec.volume_claim_templates.unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].metadata.name, "data");

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_metadata_immutability() {
    let storage = Arc::new(MemoryStorage::new());

    let statefulset = create_test_statefulset("test-immutable", "default", 3);
    let key = build_key("statefulsets", Some("default"), "test-immutable");

    // Create
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    let original_uid = created.metadata.uid.clone();

    // Try to update - UID should remain unchanged
    let mut updated_sts = created.clone();
    updated_sts.spec.replicas = Some(10);

    let updated: StatefulSet = storage.update(&key, &updated_sts).await.unwrap();
    assert_eq!(updated.metadata.uid, original_uid);
    assert_eq!(updated.spec.replicas, Some(10));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_label_selector() {
    let storage = Arc::new(MemoryStorage::new());

    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "database".to_string());
    labels.insert("tier".to_string(), "backend".to_string());

    let mut statefulset = create_test_statefulset("test-labels", "default", 3);
    statefulset.metadata.labels = Some(labels.clone());
    statefulset.spec.selector = LabelSelector {
        match_labels: Some(labels),
        match_expressions: None,
    };

    let key = build_key("statefulsets", Some("default"), "test-labels");

    // Create with labels
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert!(created.metadata.labels.is_some());
    let created_labels = created.metadata.labels.unwrap();
    assert_eq!(created_labels.get("app"), Some(&"database".to_string()));
    assert_eq!(created_labels.get("tier"), Some(&"backend".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_get_not_found() {
    let storage = Arc::new(MemoryStorage::new());

    let key = build_key("statefulsets", Some("default"), "nonexistent");
    let result = storage.get::<StatefulSet>(&key).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_statefulset_update_not_found() {
    let storage = Arc::new(MemoryStorage::new());

    let statefulset = create_test_statefulset("nonexistent", "default", 3);
    let key = build_key("statefulsets", Some("default"), "nonexistent");

    let result = storage.update(&key, &statefulset).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_statefulset_min_ready_seconds() {
    let storage = Arc::new(MemoryStorage::new());

    let mut statefulset = create_test_statefulset("test-min-ready", "default", 3);
    statefulset.spec.min_ready_seconds = Some(30);

    let key = build_key("statefulsets", Some("default"), "test-min-ready");

    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.spec.min_ready_seconds, Some(30));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_revision_history_limit() {
    let storage = Arc::new(MemoryStorage::new());

    let mut statefulset = create_test_statefulset("test-revision", "default", 3);
    statefulset.spec.revision_history_limit = Some(5);

    let key = build_key("statefulsets", Some("default"), "test-revision");

    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.spec.revision_history_limit, Some(5));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_zero_replicas() {
    let storage = Arc::new(MemoryStorage::new());

    let statefulset = create_test_statefulset("test-zero", "default", 0);
    let key = build_key("statefulsets", Some("default"), "test-zero");

    // Create with zero replicas
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.spec.replicas, Some(0));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_with_finalizers() {
    let storage = Arc::new(MemoryStorage::new());

    let mut statefulset = create_test_statefulset("test-finalizers", "default", 3);
    statefulset.metadata.finalizers = Some(vec!["finalizer.test.io".to_string()]);

    let key = build_key("statefulsets", Some("default"), "test-finalizers");

    // Create with finalizer
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(
        created.metadata.finalizers,
        Some(vec!["finalizer.test.io".to_string()])
    );

    // Verify finalizer is present
    let retrieved: StatefulSet = storage.get(&key).await.unwrap();
    assert_eq!(
        retrieved.metadata.finalizers,
        Some(vec!["finalizer.test.io".to_string()])
    );

    // Clean up - remove finalizer first
    statefulset.metadata.finalizers = None;
    storage.update(&key, &statefulset).await.unwrap();
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_statefulset_observed_generation() {
    let storage = Arc::new(MemoryStorage::new());

    let mut statefulset = create_test_statefulset("test-generation", "default", 3);
    statefulset.status = Some(StatefulSetStatus {
        replicas: 3,
        ready_replicas: Some(3),
        current_replicas: Some(3),
        updated_replicas: Some(3),
        available_replicas: Some(3),
        observed_generation: Some(5),
        collision_count: None,
        conditions: None,
        current_revision: None,
        update_revision: None,
    });

    let key = build_key("statefulsets", Some("default"), "test-generation");

    // Create with observed generation
    let created: StatefulSet = storage.create(&key, &statefulset).await.unwrap();
    assert_eq!(created.status.as_ref().unwrap().observed_generation, Some(5));

    // Clean up
    storage.delete(&key).await.unwrap();
}

//! Integration tests for ReplicaSet handler
//!
//! Tests all CRUD operations, edge cases, and error handling for replicasets

use rusternetes_common::resources::{
    Container, PodSpec, PodTemplateSpec, ReplicaSet, ReplicaSetSpec, ReplicaSetStatus,
};
use rusternetes_common::types::{LabelSelector, ObjectMeta, TypeMeta};
use rusternetes_storage::{build_key, build_prefix, memory::MemoryStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;

/// Helper function to create a test replicaset
fn create_test_replicaset(name: &str, namespace: &str, replicas: i32) -> ReplicaSet {
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), name.to_string());

    ReplicaSet {
        type_meta: TypeMeta {
            api_version: "apps/v1".to_string(),
            kind: "ReplicaSet".to_string(),
        },
        metadata: ObjectMeta {
            name: name.to_string(),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            ..Default::default()
        },
        spec: ReplicaSetSpec {
            replicas: replicas,
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                match_expressions: None,
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    name: format!("{}-pod", name),
                    namespace: Some(namespace.to_string()),
                    labels: Some(labels),
                    ..Default::default()
                }),
                spec: PodSpec {
                    containers: vec![Container {
                        name: "nginx".to_string(),
                        image: "nginx:latest".to_string(),
                        command: None,
                        args: None,
                        working_dir: None,
                        ports: None,
                        env: None,
                        resources: None,
                        volume_mounts: None,
                        image_pull_policy: None,
                        liveness_probe: None,
                        readiness_probe: None,
                        startup_probe: None,
                        restart_policy: None,
                        security_context: None,
                    }],
                    init_containers: None,
                    ephemeral_containers: None,
                    restart_policy: Some("Always".to_string()),
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
                    overhead: None,
                    topology_spread_constraints: None,
                    resource_claims: None,
                },
            },
            min_ready_seconds: Some(0),
        },
        status: Some(ReplicaSetStatus {
            replicas: 0,
            fully_labeled_replicas: Some(0),
            ready_replicas: 0,
            available_replicas: 0,
            observed_generation: Some(0),
            conditions: None,
        }),
    }
}

#[tokio::test]
async fn test_replicaset_create_and_get() {
    let storage = Arc::new(MemoryStorage::new());

    let replicaset = create_test_replicaset("test-rs", "default", 3);
    let key = build_key("replicasets", Some("default"), "test-rs");

    // Create
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert_eq!(created.metadata.name, "test-rs");
    assert_eq!(created.metadata.namespace, Some("default".to_string()));
    assert_eq!(created.spec.replicas, 3);
    assert!(!created.metadata.uid.is_empty());

    // Get
    let retrieved: ReplicaSet = storage.get(&key).await.unwrap();
    assert_eq!(retrieved.metadata.name, "test-rs");
    assert_eq!(retrieved.spec.replicas, 3);

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_update() {
    let storage = Arc::new(MemoryStorage::new());

    let mut replicaset = create_test_replicaset("test-update-rs", "default", 3);
    let key = build_key("replicasets", Some("default"), "test-update-rs");

    // Create
    storage.create(&key, &replicaset).await.unwrap();

    // Update replicas
    replicaset.spec.replicas = 5;
    let updated: ReplicaSet = storage.update(&key, &replicaset).await.unwrap();
    assert_eq!(updated.spec.replicas, 5);

    // Verify update
    let retrieved: ReplicaSet = storage.get(&key).await.unwrap();
    assert_eq!(retrieved.spec.replicas, 5);

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_delete() {
    let storage = Arc::new(MemoryStorage::new());

    let replicaset = create_test_replicaset("test-delete-rs", "default", 3);
    let key = build_key("replicasets", Some("default"), "test-delete-rs");

    // Create
    storage.create(&key, &replicaset).await.unwrap();

    // Delete
    storage.delete(&key).await.unwrap();

    // Verify deletion
    let result = storage.get::<ReplicaSet>(&key).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_replicaset_list() {
    let storage = Arc::new(MemoryStorage::new());

    // Create multiple replicasets
    let rs1 = create_test_replicaset("rs-1", "default", 3);
    let rs2 = create_test_replicaset("rs-2", "default", 5);
    let rs3 = create_test_replicaset("rs-3", "default", 2);

    let key1 = build_key("replicasets", Some("default"), "rs-1");
    let key2 = build_key("replicasets", Some("default"), "rs-2");
    let key3 = build_key("replicasets", Some("default"), "rs-3");

    storage.create(&key1, &rs1).await.unwrap();
    storage.create(&key2, &rs2).await.unwrap();
    storage.create(&key3, &rs3).await.unwrap();

    // List
    let prefix = build_prefix("replicasets", Some("default"));
    let replicasets: Vec<ReplicaSet> = storage.list(&prefix).await.unwrap();

    assert!(replicasets.len() >= 3);
    let names: Vec<String> = replicasets
        .iter()
        .map(|rs| rs.metadata.name.clone())
        .collect();
    assert!(names.contains(&"rs-1".to_string()));
    assert!(names.contains(&"rs-2".to_string()));
    assert!(names.contains(&"rs-3".to_string()));

    // Clean up
    storage.delete(&key1).await.unwrap();
    storage.delete(&key2).await.unwrap();
    storage.delete(&key3).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_list_across_namespaces() {
    let storage = Arc::new(MemoryStorage::new());

    // Create replicasets in different namespaces
    let rs1 = create_test_replicaset("rs-ns1", "namespace-1", 3);
    let rs2 = create_test_replicaset("rs-ns2", "namespace-2", 5);

    let key1 = build_key("replicasets", Some("namespace-1"), "rs-ns1");
    let key2 = build_key("replicasets", Some("namespace-2"), "rs-ns2");

    storage.create(&key1, &rs1).await.unwrap();
    storage.create(&key2, &rs2).await.unwrap();

    // List all (no namespace filter)
    let prefix = build_prefix("replicasets", None);
    let replicasets: Vec<ReplicaSet> = storage.list(&prefix).await.unwrap();

    // Should find at least our 2 replicasets
    assert!(replicasets.len() >= 2);

    // Clean up
    storage.delete(&key1).await.unwrap();
    storage.delete(&key2).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_with_status() {
    let storage = Arc::new(MemoryStorage::new());

    let mut replicaset = create_test_replicaset("test-status", "default", 3);
    replicaset.status = Some(ReplicaSetStatus {
        replicas: 3,
        fully_labeled_replicas: Some(3),
        ready_replicas: 2,
        available_replicas: 2,
        observed_generation: Some(1),
        conditions: None,
    });

    let key = build_key("replicasets", Some("default"), "test-status");

    // Create with status
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert_eq!(created.status.as_ref().unwrap().replicas, 3);
    assert_eq!(created.status.as_ref().unwrap().ready_replicas, 2);
    assert_eq!(created.status.as_ref().unwrap().available_replicas, 2);

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_with_finalizers() {
    let storage = Arc::new(MemoryStorage::new());

    let mut replicaset = create_test_replicaset("test-finalizers", "default", 3);
    replicaset.metadata.finalizers = Some(vec!["finalizer.test.io".to_string()]);

    let key = build_key("replicasets", Some("default"), "test-finalizers");

    // Create with finalizer
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert_eq!(
        created.metadata.finalizers,
        Some(vec!["finalizer.test.io".to_string()])
    );

    // Verify finalizer is present
    let retrieved: ReplicaSet = storage.get(&key).await.unwrap();
    assert_eq!(
        retrieved.metadata.finalizers,
        Some(vec!["finalizer.test.io".to_string()])
    );

    // Clean up - remove finalizer first
    replicaset.metadata.finalizers = None;
    storage.update(&key, &replicaset).await.unwrap();
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_metadata_immutability() {
    let storage = Arc::new(MemoryStorage::new());

    let replicaset = create_test_replicaset("test-immutable", "default", 3);
    let key = build_key("replicasets", Some("default"), "test-immutable");

    // Create
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    let original_uid = created.metadata.uid.clone();

    // Try to update - UID should remain unchanged
    let mut updated_rs = created.clone();
    updated_rs.spec.replicas = 10;

    let updated: ReplicaSet = storage.update(&key, &updated_rs).await.unwrap();
    assert_eq!(updated.metadata.uid, original_uid);
    assert_eq!(updated.spec.replicas, 10);

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_label_selector() {
    let storage = Arc::new(MemoryStorage::new());

    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "frontend".to_string());
    labels.insert("tier".to_string(), "web".to_string());

    let mut replicaset = create_test_replicaset("test-labels", "default", 3);
    replicaset.metadata.labels = Some(labels.clone());
    replicaset.spec.selector = LabelSelector {
        match_labels: Some(labels),
        match_expressions: None,
    };

    let key = build_key("replicasets", Some("default"), "test-labels");

    // Create with labels
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert!(created.metadata.labels.is_some());
    let created_labels = created.metadata.labels.unwrap();
    assert_eq!(created_labels.get("app"), Some(&"frontend".to_string()));
    assert_eq!(created_labels.get("tier"), Some(&"web".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_get_not_found() {
    let storage = Arc::new(MemoryStorage::new());

    let key = build_key("replicasets", Some("default"), "nonexistent");
    let result = storage.get::<ReplicaSet>(&key).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_replicaset_update_not_found() {
    let storage = Arc::new(MemoryStorage::new());

    let replicaset = create_test_replicaset("nonexistent", "default", 3);
    let key = build_key("replicasets", Some("default"), "nonexistent");

    let result = storage.update(&key, &replicaset).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_replicaset_min_ready_seconds() {
    let storage = Arc::new(MemoryStorage::new());

    let mut replicaset = create_test_replicaset("test-min-ready", "default", 3);
    replicaset.spec.min_ready_seconds = Some(30);

    let key = build_key("replicasets", Some("default"), "test-min-ready");

    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert_eq!(created.spec.min_ready_seconds, Some(30));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_zero_replicas() {
    let storage = Arc::new(MemoryStorage::new());

    let replicaset = create_test_replicaset("test-zero", "default", 0);
    let key = build_key("replicasets", Some("default"), "test-zero");

    // Create with zero replicas
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert_eq!(created.spec.replicas, 0);

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_with_owner_reference() {
    let storage = Arc::new(MemoryStorage::new());

    let mut replicaset = create_test_replicaset("test-owner", "default", 3);
    replicaset.metadata.owner_references = Some(vec![rusternetes_common::types::OwnerReference {
        api_version: "apps/v1".to_string(),
        kind: "Deployment".to_string(),
        name: "parent-deployment".to_string(),
        uid: "parent-uid-123".to_string(),
        controller: Some(true),
        block_owner_deletion: Some(true),
    }]);

    let key = build_key("replicasets", Some("default"), "test-owner");

    // Create with owner reference
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert!(created.metadata.owner_references.is_some());
    let owner_refs = created.metadata.owner_references.unwrap();
    assert_eq!(owner_refs.len(), 1);
    assert_eq!(owner_refs[0].kind, "Deployment");
    assert_eq!(owner_refs[0].name, "parent-deployment");

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_replicaset_observed_generation() {
    let storage = Arc::new(MemoryStorage::new());

    let mut replicaset = create_test_replicaset("test-generation", "default", 3);
    replicaset.status = Some(ReplicaSetStatus {
        replicas: 3,
        fully_labeled_replicas: Some(3),
        ready_replicas: 3,
        available_replicas: 3,
        observed_generation: Some(5),
        conditions: None,
    });

    let key = build_key("replicasets", Some("default"), "test-generation");

    // Create with observed generation
    let created: ReplicaSet = storage.create(&key, &replicaset).await.unwrap();
    assert_eq!(
        created.status.as_ref().unwrap().observed_generation,
        Some(5)
    );

    // Clean up
    storage.delete(&key).await.unwrap();
}

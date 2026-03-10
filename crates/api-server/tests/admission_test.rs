// Integration tests for admission controllers (LimitRange and ResourceQuota)
//
// These tests use in-memory storage and don't require a running etcd instance.
// The admission controllers are tested through the functions which:
// 1. apply_limit_range() - applies defaults and validates constraints
// 2. check_resource_quota() - ensures pod doesn't exceed quota
//
// Full E2E testing happens through the workflow tests that test the pod creation handler.

use rusternetes_api_server::admission::{apply_limit_range, check_resource_quota};
use rusternetes_common::resources::{
    Container, LimitRange, LimitRangeItem, LimitRangeSpec, Pod, PodSpec, ResourceQuota,
    ResourceQuotaSpec,
};
use rusternetes_common::types::{ObjectMeta, TypeMeta};
use rusternetes_storage::{build_key, memory::MemoryStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;

fn create_minimal_pod(name: &str, namespace: &str) -> Pod {
    Pod {
        type_meta: TypeMeta {
            kind: "Pod".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: ObjectMeta::new(name).with_namespace(namespace),
        spec: Some(PodSpec {
            containers: vec![Container {
                name: "test-container".to_string(),
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
                security_context: None,
            }],
            init_containers: None,
            volumes: None,
            restart_policy: None,
            node_name: None,
            node_selector: None,
            service_account_name: None,
            hostname: None,
            host_network: None,
            host_pid: None,
            host_ipc: None,
            affinity: None,
            tolerations: None,
            priority_class_name: None,
            priority: None,
        }),
        status: None,
    }
}

#[tokio::test]
async fn test_limit_range_allows_when_no_limit_exists() {
    let storage = Arc::new(MemoryStorage::new());
    let mut pod = create_minimal_pod("test-pod", "test-namespace");

    let result = apply_limit_range(&storage, "test-namespace", &mut pod).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "Pod should be allowed when no LimitRange exists");
}

#[tokio::test]
async fn test_quota_allows_when_no_quota_exists() {
    let storage = Arc::new(MemoryStorage::new());
    let pod = create_minimal_pod("test-pod", "test-namespace");

    let result = check_resource_quota(&storage, "test-namespace", &pod).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "Pod should be allowed when no quota exists");
}

#[tokio::test]
async fn test_limit_range_applies_defaults() {
    let storage = Arc::new(MemoryStorage::new());

    // Create a LimitRange with defaults
    let mut default_limits = HashMap::new();
    default_limits.insert("cpu".to_string(), "500m".to_string());
    default_limits.insert("memory".to_string(), "512Mi".to_string());

    let limit_range = LimitRange {
        type_meta: TypeMeta {
            kind: "LimitRange".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: ObjectMeta::new("test-limit-range").with_namespace("test-namespace"),
        spec: LimitRangeSpec {
            limits: vec![LimitRangeItem {
                item_type: "Container".to_string(),
                max: None,
                min: None,
                default: Some(default_limits.clone()),
                default_request: None,
                max_limit_request_ratio: None,
            }],
        },
    };

    let key = build_key("limitranges", Some("test-namespace"), "test-limit-range");
    storage.create(&key, &limit_range).await.unwrap();

    // Create a pod without resource limits
    let mut pod = create_minimal_pod("test-pod", "test-namespace");

    // Apply limit range
    let result = apply_limit_range(&storage, "test-namespace", &mut pod).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "LimitRange admission should pass");

    // Verify defaults were applied
    let container = &pod.spec.unwrap().containers[0];
    assert!(container.resources.is_some());
    let resources = container.resources.as_ref().unwrap();
    assert!(resources.limits.is_some());
    let limits = resources.limits.as_ref().unwrap();
    assert_eq!(limits.get("cpu").unwrap(), "500m");
    assert_eq!(limits.get("memory").unwrap(), "512Mi");
}

#[tokio::test]
async fn test_quota_allows_when_under_limit() {
    let storage = Arc::new(MemoryStorage::new());

    // Create a quota
    let mut hard = HashMap::new();
    hard.insert("pods".to_string(), "10".to_string());

    let quota = ResourceQuota::new(
        "test-quota",
        "test-namespace",
        ResourceQuotaSpec {
            hard: Some(hard),
            scopes: None,
            scope_selector: None,
        },
    );

    let key = build_key("resourcequotas", Some("test-namespace"), "test-quota");
    storage.create(&key, &quota).await.unwrap();

    // Try to create a pod
    let pod = create_minimal_pod("test-pod", "test-namespace");

    let result = check_resource_quota(&storage, "test-namespace", &pod).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "Pod should be allowed under quota");
}

#[tokio::test]
async fn test_quota_rejects_when_exceeding_pod_count() {
    let storage = Arc::new(MemoryStorage::new());

    // Create a quota with low pod limit
    let mut hard = HashMap::new();
    hard.insert("pods".to_string(), "1".to_string());

    let quota = ResourceQuota::new(
        "test-quota",
        "test-namespace",
        ResourceQuotaSpec {
            hard: Some(hard),
            scopes: None,
            scope_selector: None,
        },
    );

    let quota_key = build_key("resourcequotas", Some("test-namespace"), "test-quota");
    storage.create(&quota_key, &quota).await.unwrap();

    // Create an existing pod
    let existing_pod = create_minimal_pod("existing-pod", "test-namespace");
    let pod_key = build_key("pods", Some("test-namespace"), "existing-pod");
    storage.create(&pod_key, &existing_pod).await.unwrap();

    // Try to create a second pod (should exceed quota)
    let new_pod = create_minimal_pod("new-pod", "test-namespace");

    let result = check_resource_quota(&storage, "test-namespace", &new_pod).await;
    assert!(result.is_ok());
    assert!(!result.unwrap(), "Pod should be rejected for exceeding pod count quota");
}

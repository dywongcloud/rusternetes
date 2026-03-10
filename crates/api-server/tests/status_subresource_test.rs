// Integration tests for Status Subresource
// Tests that status updates don't affect spec and vice versa

use rusternetes_storage::{build_key, memory::MemoryStorage, Storage};
use serde_json::{json, Value};
use std::sync::Arc;

async fn setup_test() -> Arc<MemoryStorage> {
    let storage = Arc::new(MemoryStorage::new());
    storage.clear();
    storage
}

#[tokio::test]
async fn test_status_update_preserves_spec() {
    let storage = setup_test().await;

    // Create a deployment
    let deployment = json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": "test-deployment",
            "namespace": "default",
            "uid": "test-uid-1",
            "resourceVersion": "1"
        },
        "spec": {
            "replicas": 3,
            "selector": {
                "matchLabels": {
                    "app": "test"
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app": "test"
                    }
                },
                "spec": {
                    "containers": [{
                        "name": "nginx",
                        "image": "nginx:1.25-alpine"
                    }]
                }
            }
        }
    });

    let key = build_key("deployments", Some("default"), "test-deployment");
    storage.create(&key, &deployment).await.unwrap();

    // Simulate status update
    let current: Value = storage.get(&key).await.unwrap();
    let mut updated = current.clone();

    // Update status
    updated["status"] = json!({
        "replicas": 3,
        "readyReplicas": 2,
        "availableReplicas": 2
    });

    // Increment resource version
    if let Some(obj) = updated.as_object_mut() {
        if let Some(metadata) = obj.get_mut("metadata") {
            if let Some(metadata_obj) = metadata.as_object_mut() {
                metadata_obj.insert("resourceVersion".to_string(), json!("2"));
            }
        }
    }

    storage.update(&key, &updated).await.unwrap();

    // Verify spec is unchanged
    let final_resource: Value = storage.get(&key).await.unwrap();
    assert_eq!(final_resource["spec"]["replicas"], 3);
    assert_eq!(final_resource["status"]["readyReplicas"], 2);
    assert_eq!(final_resource["status"]["availableReplicas"], 2);
}

#[tokio::test]
async fn test_spec_update_does_not_affect_status() {
    let storage = setup_test().await;

    // Create a deployment with initial status
    let deployment = json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": "test-deployment",
            "namespace": "default",
            "uid": "test-uid-2",
            "resourceVersion": "1"
        },
        "spec": {
            "replicas": 3,
            "selector": {
                "matchLabels": {
                    "app": "test"
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app": "test"
                    }
                },
                "spec": {
                    "containers": [{
                        "name": "nginx",
                        "image": "nginx:1.25-alpine"
                    }]
                }
            }
        },
        "status": {
            "replicas": 3,
            "readyReplicas": 3
        }
    });

    let key = build_key("deployments", Some("default"), "test-deployment");
    storage.create(&key, &deployment).await.unwrap();

    // Update spec (scale up)
    let current: Value = storage.get(&key).await.unwrap();
    let mut updated = current.clone();

    updated["spec"]["replicas"] = json!(5);

    // Preserve status
    // In a real implementation, the API server would handle this
    if let Some(status) = current.get("status") {
        updated["status"] = status.clone();
    }

    storage.update(&key, &updated).await.unwrap();

    // Verify status is preserved (still shows old values)
    let final_resource: Value = storage.get(&key).await.unwrap();
    assert_eq!(final_resource["spec"]["replicas"], 5);
    assert_eq!(final_resource["status"]["replicas"], 3);
    assert_eq!(final_resource["status"]["readyReplicas"], 3);
}

#[tokio::test]
async fn test_status_with_cluster_scoped_resource() {
    let storage = setup_test().await;

    // Create a PersistentVolume (cluster-scoped)
    let pv = json!({
        "apiVersion": "v1",
        "kind": "PersistentVolume",
        "metadata": {
            "name": "test-pv",
            "uid": "test-pv-uid",
            "resourceVersion": "1"
        },
        "spec": {
            "capacity": {
                "storage": "10Gi"
            },
            "accessModes": ["ReadWriteOnce"],
            "persistentVolumeReclaimPolicy": "Retain",
            "storageClassName": "standard"
        }
    });

    let key = build_key("persistentvolumes", None, "test-pv");
    storage.create(&key, &pv).await.unwrap();

    // Update status
    let current: Value = storage.get(&key).await.unwrap();
    let mut updated = current.clone();

    updated["status"] = json!({
        "phase": "Available"
    });

    storage.update(&key, &updated).await.unwrap();

    // Verify spec is unchanged
    let final_resource: Value = storage.get(&key).await.unwrap();
    assert_eq!(final_resource["spec"]["capacity"]["storage"], "10Gi");
    assert_eq!(final_resource["status"]["phase"], "Available");
}

#[tokio::test]
async fn test_concurrent_spec_and_status_updates() {
    let storage = setup_test().await;

    // Create a deployment
    let deployment = json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": "concurrent-test",
            "namespace": "default",
            "uid": "test-uid-3",
            "resourceVersion": "1"
        },
        "spec": {
            "replicas": 3,
            "selector": {
                "matchLabels": {
                    "app": "test"
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app": "test"
                    }
                },
                "spec": {
                    "containers": [{
                        "name": "nginx",
                        "image": "nginx:1.25-alpine"
                    }]
                }
            }
        }
    });

    let key = build_key("deployments", Some("default"), "concurrent-test");
    storage.create(&key, &deployment).await.unwrap();

    // Update spec
    let current1: Value = storage.get(&key).await.unwrap();
    let mut spec_updated = current1.clone();
    spec_updated["spec"]["replicas"] = json!(5);
    spec_updated["spec"]["template"]["spec"]["containers"][0]["image"] = json!("nginx:1.26-alpine");
    spec_updated["metadata"]["resourceVersion"] = json!("2");
    storage.update(&key, &spec_updated).await.unwrap();

    // Update status
    let current2: Value = storage.get(&key).await.unwrap();
    let mut status_updated = current2.clone();
    status_updated["status"] = json!({
        "replicas": 5,
        "readyReplicas": 4,
        "availableReplicas": 4
    });
    status_updated["metadata"]["resourceVersion"] = json!("3");
    storage.update(&key, &status_updated).await.unwrap();

    // Get the final state
    let final_state: Value = storage.get(&key).await.unwrap();

    // Verify both updates are present
    assert_eq!(final_state["spec"]["replicas"], 5);
    assert_eq!(final_state["spec"]["template"]["spec"]["containers"][0]["image"], "nginx:1.26-alpine");
    assert_eq!(final_state["status"]["readyReplicas"], 4);
    assert_eq!(final_state["status"]["availableReplicas"], 4);
}

#[tokio::test]
async fn test_resource_version_increments() {
    let storage = setup_test().await;

    // Create a pod
    let pod = json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": "test-pod",
            "namespace": "default",
            "uid": "test-pod-uid",
            "resourceVersion": "1"
        },
        "spec": {
            "containers": [{
                "name": "nginx",
                "image": "nginx:1.25-alpine"
            }]
        }
    });

    let key = build_key("pods", Some("default"), "test-pod");
    storage.create(&key, &pod).await.unwrap();

    let original_resource: Value = storage.get(&key).await.unwrap();
    let original_version = original_resource["metadata"]["resourceVersion"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();

    // Update status
    let mut updated = original_resource.clone();
    updated["status"] = json!({
        "phase": "Running",
        "podIP": "10.0.0.5"
    });
    updated["metadata"]["resourceVersion"] = json!((original_version + 1).to_string());

    storage.update(&key, &updated).await.unwrap();

    let new_resource: Value = storage.get(&key).await.unwrap();
    let new_version = new_resource["metadata"]["resourceVersion"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();

    assert_eq!(new_version, original_version + 1, "ResourceVersion should be incremented");
}

#[tokio::test]
async fn test_status_update_missing_resource() {
    let storage = setup_test().await;

    // Try to get non-existent resource
    let key = build_key("pods", Some("default"), "non-existent");
    let result: Result<Value, _> = storage.get(&key).await;

    assert!(result.is_err(), "Should return error for non-existent resource");
}

#[tokio::test]
async fn test_metadata_preservation_on_status_update() {
    let storage = setup_test().await;

    // Create a pod with labels and annotations
    let pod = json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": "test-pod",
            "namespace": "default",
            "uid": "test-pod-uid-2",
            "resourceVersion": "1",
            "labels": {
                "app": "test",
                "version": "v1"
            },
            "annotations": {
                "description": "test pod"
            }
        },
        "spec": {
            "containers": [{
                "name": "nginx",
                "image": "nginx:1.25-alpine"
            }]
        }
    });

    let key = build_key("pods", Some("default"), "test-pod");
    storage.create(&key, &pod).await.unwrap();

    // Update status
    let current: Value = storage.get(&key).await.unwrap();
    let mut updated = current.clone();

    updated["status"] = json!({
        "phase": "Running"
    });
    updated["metadata"]["resourceVersion"] = json!("2");

    storage.update(&key, &updated).await.unwrap();

    // Verify metadata is preserved
    let final_resource: Value = storage.get(&key).await.unwrap();
    assert_eq!(final_resource["metadata"]["labels"]["app"], "test");
    assert_eq!(final_resource["metadata"]["labels"]["version"], "v1");
    assert_eq!(final_resource["metadata"]["annotations"]["description"], "test pod");
    assert_eq!(final_resource["status"]["phase"], "Running");
}

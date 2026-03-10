# Rusternetes Testing Implementation Guide

This document provides comprehensive templates and guidance for implementing tests across all Rusternetes components.

## Test Infrastructure Created

### Helper Modules (`tests/common/`)

1. **etcd_helper.rs** - Etcd test utilities
   - `create_test_storage()` - Create test etcd connection
   - `cleanup_test_data()` - Clean up all test resources

2. **cluster_helper.rs** - Cluster setup utilities
   - `create_test_node()` - Create test node in etcd

3. **fixture_helper.rs** - Resource creation utilities
   - `create_test_namespace()`
   - `create_test_pvc()`
   - `create_test_pv()`
   - `create_test_storage_class()`
   - `create_test_volume_snapshot_class()`
   - `create_test_volume_snapshot()`
   - `create_test_deployment()`

4. **assertion_helper.rs** - Custom assertions
   - `assert_pvc_bound()`
   - `assert_pv_bound()`
   - `assert_snapshot_ready()`
   - `assert_deployment_replicas()`
   - `assert_pod_labels()`

## Test Categories

### 1. Unit Tests (Controller Logic)

Location: Within each controller file (`#[cfg(test)] mod tests`)

**Volume Snapshot Controller** (`crates/controller-manager/src/controllers/volume_snapshot.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_driver_supported() {
        // ✅ Implemented
    }

    #[test]
    fn test_create_snapshot_content_structure() {
        // ✅ Implemented
    }

    #[test]
    fn test_create_snapshot_content_with_retain_policy() {
        // ✅ Implemented
    }

    #[test]
    fn test_snapshot_handle_uniqueness() {
        // ✅ Implemented
    }

    // TODO: Add more unit tests
    #[test]
    fn test_validates_pvc_source() {
        // Test that snapshot fails without valid PVC source
    }

    #[test]
    fn test_respects_deletion_policy() {
        // Test both Delete and Retain policies
    }
}
```

**Template for Other Controllers:**

```rust
// crates/controller-manager/src/controllers/<controller>.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_<specific_logic>() {
        // Setup
        let storage = Arc::new(unsafe { std::mem::zeroed() });
        let controller = <Controller>::new(storage);

        // Exercise
        let result = controller.<method>(...);

        // Verify
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
}
```

### 2. Integration Tests (With Etcd)

Location: `crates/<component>/tests/`

**Volume Snapshot Integration Test** (`crates/controller-manager/tests/volume_snapshot_controller_test.rs`)

```rust
// ✅ Created with test_snapshot_content_auto_creation()

// TODO: Complete the following tests:

#[tokio::test]
#[ignore] // Requires etcd
async fn test_snapshot_deletion_with_delete_policy() {
    let storage = setup_test().await;

    // Create snapshot with Delete policy
    // Delete the VolumeSnapshot
    // Verify VolumeSnapshotContent is also deleted
}

#[tokio::test]
#[ignore]
async fn test_snapshot_deletion_with_retain_policy() {
    let storage = setup_test().await;

    // Create snapshot with Retain policy
    // Delete the VolumeSnapshot
    // Verify VolumeSnapshotContent still exists
}

#[tokio::test]
#[ignore]
async fn test_snapshot_without_bound_pvc_fails() {
    let storage = setup_test().await;

    // Create snapshot referencing unbound PVC
    // Verify no VolumeSnapshotContent is created
    // Verify snapshot status shows error
}

#[tokio::test]
#[ignore]
async fn test_snapshot_with_invalid_class_fails() {
    let storage = setup_test().await;

    // Create snapshot with non-existent VolumeSnapshotClass
    // Verify operation fails gracefully
}
```

**Dynamic Provisioner Tests** (Create: `crates/controller-manager/tests/dynamic_provisioner_test.rs`)

```rust
use rusternetes_controller_manager::controllers::dynamic_provisioner::DynamicProvisionerController;

#[tokio::test]
#[ignore]
async fn test_provisions_pv_for_pvc_with_storageclass() {
    let storage = setup_test().await;

    // Create StorageClass
    let sc = create_test_storage_class(&storage, "fast", "rusternetes.io/hostpath").await;

    // Create PVC with storageClassName
    let pvc = create_test_pvc(&storage, "test-pvc", "default", Some("fast")).await;

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();

    // Verify PV was created
    let pv_key = build_key("persistentvolumes", None, "pvc-default-test-pvc");
    let pv: PersistentVolume = storage.get(&pv_key).await.unwrap();

    assert_eq!(pv.spec.storage_class_name, Some("fast".to_string()));
}

#[tokio::test]
#[ignore]
async fn test_skips_pvc_without_storageclass() {
    // Create PVC without storageClassName
    // Verify no PV is created
}

#[tokio::test]
#[ignore]
async fn test_skips_already_bound_pvcs() {
    // Create bound PVC
    // Verify provisioner skips it
}

#[tokio::test]
#[ignore]
async fn test_honors_storage_capacity() {
    // Create PVC requesting 5Gi
    // Verify PV has capacity 5Gi
}

#[tokio::test]
#[ignore]
async fn test_honors_access_modes() {
    // Create PVC with ReadWriteOnce
    // Verify PV has same access mode
}

#[tokio::test]
#[ignore]
async fn test_honors_reclaim_policy() {
    // Test both Delete and Retain policies
}
```

**PV Binder Tests** (Create: `crates/controller-manager/tests/pv_binder_test.rs`)

```rust
use rusternetes_controller_manager::controllers::pv_binder::PVBinderController;

#[tokio::test]
#[ignore]
async fn test_binds_matching_pv_to_pvc() {
    let storage = setup_test().await;

    // Create available PV
    let pv = create_test_pv(&storage, "test-pv", Some("fast"), 10).await;

    // Create pending PVC
    let pvc = create_test_pvc(&storage, "test-pvc", "default", Some("fast")).await;

    // Run binder
    let controller = PVBinderController::new(storage.clone());
    controller.reconcile().await.unwrap();

    // Verify binding
    let updated_pv: PersistentVolume = storage.get(&build_key("persistentvolumes", None, "test-pv")).await.unwrap();
    let updated_pvc: PersistentVolumeClaim = storage.get(&build_key("persistentvolumeclaims", Some("default"), "test-pvc")).await.unwrap();

    assert_pv_bound(&updated_pv, "test-pvc", "default");
    assert_pvc_bound(&updated_pvc);
}

#[tokio::test]
#[ignore]
async fn test_matches_storage_class() {
    // PV and PVC with matching storage class should bind
    // PV and PVC with different storage class should not bind
}

#[tokio::test]
#[ignore]
async fn test_matches_capacity() {
    // PV with sufficient capacity should bind
    // PV with insufficient capacity should not bind
}

#[tokio::test]
#[ignore]
async fn test_matches_access_modes() {
    // PV supporting PVC's access modes should bind
}
```

**Deployment Controller Tests** (Create: `crates/controller-manager/tests/deployment_controller_test.rs`)

```rust
use rusternetes_controller_manager::controllers::deployment::DeploymentController;

#[tokio::test]
#[ignore]
async fn test_deployment_creates_pods() {
    let storage = setup_test().await;

    // Create deployment with replicas: 3
    let deployment = create_test_deployment(&storage, "nginx", "default", 3).await;

    // Run controller
    let controller = DeploymentController::new(storage.clone(), 10);
    controller.reconcile().await.unwrap();

    // Verify 3 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
    assert_eq!(pods.len(), 3);

    // Verify pods have correct labels
    for pod in pods {
        assert_pod_labels(&pod, &deployment.spec.template.metadata.unwrap().labels.unwrap());
    }
}

#[tokio::test]
#[ignore]
async fn test_deployment_scales_up() {
    // Create deployment with 2 replicas
    // Update to 5 replicas
    // Verify 3 new pods created
}

#[tokio::test]
#[ignore]
async fn test_deployment_scales_down() {
    // Create deployment with 5 replicas
    // Update to 2 replicas
    // Verify 3 pods deleted
}

#[tokio::test]
#[ignore]
async fn test_deployment_self_healing() {
    // Create deployment
    // Delete one pod
    // Verify new pod is created
}
```

### 3. End-to-End Workflow Tests

Location: `crates/api-server/tests/e2e_workflow_test.rs`

```rust
#[tokio::test]
#[ignore]
async fn test_complete_pod_lifecycle() {
    // 1. Create namespace
    // 2. Create pod via API
    // 3. Verify pod stored in etcd
    // 4. Simulate scheduler assignment
    // 5. Verify pod has nodeName
    // 6. Simulate kubelet updating status
    // 7. Verify pod phase is Running
}

#[tokio::test]
#[ignore]
async fn test_deployment_workflow() {
    // 1. Create deployment
    // 2. Controller creates pods
    // 3. Scheduler assigns pods
    // 4. Kubelet runs containers
    // 5. Verify all pods running
}

#[tokio::test]
#[ignore]
async fn test_dynamic_pvc_workflow() {
    // 1. Create StorageClass
    // 2. Create PVC
    // 3. Dynamic provisioner creates PV
    // 4. PV binder binds PVC to PV
    // 5. Pod uses PVC
    // 6. Verify volume mounted
}

#[tokio::test]
#[ignore]
async fn test_snapshot_workflow() {
    // 1. Create VolumeSnapshotClass
    // 2. Create PVC with data
    // 3. Create VolumeSnapshot
    // 4. Controller creates VolumeSnapshotContent
    // 5. Verify snapshot ready
    // 6. Delete snapshot with Delete policy
    // 7. Verify content also deleted
}
```

### 4. API Handler Tests

Each handler should have tests in `crates/api-server/tests/<resource>_integration_test.rs`

**Template:**

```rust
use rusternetes_api_server::*;

#[tokio::test]
async fn test_create_<resource>() {
    // POST request
    // Verify 201 Created
    // Verify resource in etcd
}

#[tokio::test]
async fn test_get_<resource>() {
    // Create resource
    // GET request
    // Verify 200 OK with correct data
}

#[tokio::test]
async fn test_list_<resource>s() {
    // Create multiple resources
    // GET list endpoint
    // Verify all returned
}

#[tokio::test]
async fn test_update_<resource>() {
    // Create resource
    // PUT request with changes
    // Verify resource updated
}

#[tokio::test]
async fn test_delete_<resource>() {
    // Create resource
    // DELETE request
    // Verify 204 No Content
    // Verify resource deleted from etcd
}

#[tokio::test]
async fn test_create_with_invalid_data() {
    // POST with invalid JSON
    // Verify 400 Bad Request
}

#[tokio::test]
async fn test_authorization_denied() {
    // Request without proper RBAC
    // Verify 403 Forbidden
}
```

### 5. Scheduler Tests

Location: `crates/scheduler/tests/scheduler_test.rs`

```rust
use rusternetes_scheduler::*;

#[tokio::test]
#[ignore]
async fn test_node_affinity_required() {
    // Pod with required node affinity
    // Only matching nodes should be candidates
}

#[tokio::test]
#[ignore]
async fn test_node_affinity_preferred() {
    // Pod with preferred node affinity
    // Matching nodes should score higher
}

#[tokio::test]
#[ignore]
async fn test_match_expressions_operators() {
    // Test In, NotIn, Exists, DoesNotExist operators
}

#[tokio::test]
#[ignore]
async fn test_taints_and_tolerations() {
    // Node with taints
    // Pod without toleration should not schedule
    // Pod with toleration should schedule
}

#[tokio::test]
#[ignore]
async fn test_resource_based_scheduling() {
    // Nodes with different resources
    // Pod should schedule on node with sufficient resources
}
```

### 6. Kubelet Runtime Tests

Location: `crates/kubelet/tests/runtime_test.rs`

```rust
#[tokio::test]
#[ignore]
async fn test_volume_creation_emptydir() {
    // Create pod with emptyDir volume
    // Verify directory created
    // Verify mounted in container
}

#[tokio::test]
#[ignore]
async fn test_orphaned_container_cleanup() {
    // Create container
    // Delete pod from etcd
    // Run cleanup
    // Verify container removed
}

#[tokio::test]
#[ignore]
async fn test_pod_ip_tracking() {
    // Start container
    // Verify pod IP populated
}

#[tokio::test]
#[ignore]
async fn test_restart_count_tracking() {
    // Container crashes
    // Verify restart count incremented
}
```

### 7. Authentication & Authorization Tests

Location: `crates/api-server/tests/auth_test.rs`

```rust
#[tokio::test]
async fn test_jwt_token_validation() {
    // Valid JWT should authenticate
    // Invalid JWT should reject
}

#[tokio::test]
async fn test_rbac_permissions() {
    // User with role can access resources
    // User without role cannot access
}

#[tokio::test]
async fn test_skip_auth_mode() {
    // In skip-auth mode, all requests succeed
}
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Module

```bash
cargo test --test volume_snapshot_controller_test
```

### Run Integration Tests (Requires Etcd)

```bash
# Start etcd
podman-compose up -d etcd

# Run ignored tests
cargo test -- --ignored

# Or run specific test
cargo test --test volume_snapshot_controller_test -- --ignored
```

### Run Unit Tests Only

```bash
cargo test --lib
```

## Test Coverage Goals

- **Unit Tests**: 80%+ coverage of controller logic
- **Integration Tests**: All critical workflows tested
- **E2E Tests**: Complete user journeys verified
- **API Tests**: All endpoints with success and failure cases

## Next Steps

1. **Implement Volume Snapshot Integration Tests** (Priority 1)
   - Complete the test_snapshot_deletion_* tests
   - Add error case tests

2. **Implement Controller Tests** (Priority 2)
   - Dynamic Provisioner
   - PV Binder
   - Deployment Controller

3. **Implement E2E Tests** (Priority 3)
   - Complete workflows from API to running workload

4. **Add Performance Tests** (Priority 4)
   - Load testing
   - Scalability testing

## Test Template Files

All test templates are available in:
- `tests/common/` - Helper utilities
- Individual test files use `#[ignore]` for tests requiring etcd
- Use `setup_test()` helper for test isolation

Remember to run `cargo fmt` and `cargo clippy` before committing tests!

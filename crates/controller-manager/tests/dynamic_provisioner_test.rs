// Integration tests for Dynamic Provisioner Controller
// Note: These tests use in-memory storage for fast, isolated testing

use rusternetes_common::resources::volume::*;
use rusternetes_common::resources::*;
use rusternetes_common::types::{ObjectMeta, TypeMeta};
use rusternetes_controller_manager::controllers::dynamic_provisioner::DynamicProvisionerController;
use rusternetes_storage::{build_key, memory::MemoryStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn setup_test() -> Arc<MemoryStorage> {
    let storage = Arc::new(MemoryStorage::new());

    // Clean up test data




    storage
}

#[tokio::test]
async fn test_provisions_pv_for_pvc_with_storageclass() {
    let storage = setup_test().await;

    // Create StorageClass
    let sc = StorageClass {
        type_meta: TypeMeta {
            kind: "StorageClass".to_string(),
            api_version: "storage.k8s.io/v1".to_string(),
        },
        metadata: ObjectMeta::new("fast"),
        provisioner: "rusternetes.io/hostpath".to_string(),
        parameters: Some({
            let mut params = HashMap::new();
            params.insert("path".to_string(), "/tmp/rusternetes/test-dynamic-pvs".to_string());
            params
        }),
        reclaim_policy: Some(PersistentVolumeReclaimPolicy::Delete),
        volume_binding_mode: Some(VolumeBindingMode::Immediate),
        allowed_topologies: None,
        allow_volume_expansion: None,
    };

    let sc_key = build_key("storageclasses", None, "fast");
    storage.create(&sc_key, &sc).await.unwrap();

    // Create PVC with storageClassName
    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "5Gi".to_string());

    let pvc = PersistentVolumeClaim {
        type_meta: TypeMeta {
            kind: "PersistentVolumeClaim".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("test-pvc");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
            resources: ResourceRequirements {
                limits: None,
                requests: Some(requests),
            },
            volume_name: None,
            storage_class_name: Some("fast".to_string()),
            volume_mode: Some(PersistentVolumeMode::Filesystem),
            selector: None,
            data_source: None,
        },
        status: Some(PersistentVolumeClaimStatus {
            phase: PersistentVolumeClaimPhase::Pending,
            access_modes: None,
            capacity: None,
            conditions: None,
        }),
    };

    let pvc_key = build_key("persistentvolumeclaims", Some("default"), "test-pvc");
    storage.create(&pvc_key, &pvc).await.unwrap();

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify PV was created
    let pv_name = "pvc-default-test-pvc";
    let pv_key = build_key("persistentvolumes", None, pv_name);
    let pv: Result<PersistentVolume, _> = storage.get(&pv_key).await;

    assert!(pv.is_ok(), "PV should be created by dynamic provisioner");
    let pv = pv.unwrap();

    assert_eq!(pv.spec.storage_class_name, Some("fast".to_string()));
    assert_eq!(pv.spec.capacity.get("storage"), Some(&"5Gi".to_string()));
    assert_eq!(pv.spec.access_modes, vec![PersistentVolumeAccessMode::ReadWriteOnce]);
    assert_eq!(pv.spec.persistent_volume_reclaim_policy, Some(PersistentVolumeReclaimPolicy::Delete));
}

#[tokio::test]
async fn test_skips_pvc_without_storageclass() {
    let storage = setup_test().await;

    // Create PVC without storageClassName
    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "5Gi".to_string());

    let pvc = PersistentVolumeClaim {
        type_meta: TypeMeta {
            kind: "PersistentVolumeClaim".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("no-sc-pvc");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
            resources: ResourceRequirements {
                limits: None,
                requests: Some(requests),
            },
            volume_name: None,
            storage_class_name: None, // No storage class
            volume_mode: Some(PersistentVolumeMode::Filesystem),
            selector: None,
            data_source: None,
        },
        status: Some(PersistentVolumeClaimStatus {
            phase: PersistentVolumeClaimPhase::Pending,
            access_modes: None,
            capacity: None,
            conditions: None,
        }),
    };

    let pvc_key = build_key("persistentvolumeclaims", Some("default"), "no-sc-pvc");
    storage.create(&pvc_key, &pvc).await.unwrap();

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify no PV was created
    let pv_name = "pvc-default-no-sc-pvc";
    let pv_key = build_key("persistentvolumes", None, pv_name);
    let pv: Result<PersistentVolume, _> = storage.get(&pv_key).await;

    assert!(pv.is_err(), "PV should not be created for PVC without storage class");
}

#[tokio::test]
async fn test_skips_already_bound_pvcs() {
    let storage = setup_test().await;

    // Create StorageClass
    let sc = StorageClass {
        type_meta: TypeMeta {
            kind: "StorageClass".to_string(),
            api_version: "storage.k8s.io/v1".to_string(),
        },
        metadata: ObjectMeta::new("fast"),
        provisioner: "rusternetes.io/hostpath".to_string(),
        parameters: None,
        reclaim_policy: Some(PersistentVolumeReclaimPolicy::Delete),
        volume_binding_mode: Some(VolumeBindingMode::Immediate),
        allowed_topologies: None,
        allow_volume_expansion: None,
    };

    let sc_key = build_key("storageclasses", None, "fast");
    storage.create(&sc_key, &sc).await.unwrap();

    // Create already bound PVC
    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "5Gi".to_string());

    let pvc = PersistentVolumeClaim {
        type_meta: TypeMeta {
            kind: "PersistentVolumeClaim".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("bound-pvc");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
            resources: ResourceRequirements {
                limits: None,
                requests: Some(requests),
            },
            volume_name: Some("existing-pv".to_string()), // Already bound
            storage_class_name: Some("fast".to_string()),
            volume_mode: Some(PersistentVolumeMode::Filesystem),
            selector: None,
            data_source: None,
        },
        status: Some(PersistentVolumeClaimStatus {
            phase: PersistentVolumeClaimPhase::Bound,
            access_modes: Some(vec![PersistentVolumeAccessMode::ReadWriteOnce]),
            capacity: Some({
                let mut cap = HashMap::new();
                cap.insert("storage".to_string(), "5Gi".to_string());
                cap
            }),
            conditions: None,
        }),
    };

    let pvc_key = build_key("persistentvolumeclaims", Some("default"), "bound-pvc");
    storage.create(&pvc_key, &pvc).await.unwrap();

    // Count existing PVs before provisioning
    let pvs_before: Vec<PersistentVolume> = storage.list("/registry/persistentvolumes/").await.unwrap_or_default();
    let count_before = pvs_before.len();

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify no new PV was created
    let pvs_after: Vec<PersistentVolume> = storage.list("/registry/persistentvolumes/").await.unwrap_or_default();
    let count_after = pvs_after.len();

    assert_eq!(count_after, count_before, "No new PV should be created for already bound PVC");
}

#[tokio::test]
async fn test_honors_storage_capacity() {
    let storage = setup_test().await;

    // Create StorageClass
    let sc = StorageClass {
        type_meta: TypeMeta {
            kind: "StorageClass".to_string(),
            api_version: "storage.k8s.io/v1".to_string(),
        },
        metadata: ObjectMeta::new("fast"),
        provisioner: "rusternetes.io/hostpath".to_string(),
        parameters: None,
        reclaim_policy: Some(PersistentVolumeReclaimPolicy::Delete),
        volume_binding_mode: Some(VolumeBindingMode::Immediate),
        allowed_topologies: None,
        allow_volume_expansion: None,
    };

    let sc_key = build_key("storageclasses", None, "fast");
    storage.create(&sc_key, &sc).await.unwrap();

    // Create PVC requesting 10Gi
    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "10Gi".to_string());

    let pvc = PersistentVolumeClaim {
        type_meta: TypeMeta {
            kind: "PersistentVolumeClaim".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("capacity-test-pvc");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
            resources: ResourceRequirements {
                limits: None,
                requests: Some(requests),
            },
            volume_name: None,
            storage_class_name: Some("fast".to_string()),
            volume_mode: Some(PersistentVolumeMode::Filesystem),
            selector: None,
            data_source: None,
        },
        status: Some(PersistentVolumeClaimStatus {
            phase: PersistentVolumeClaimPhase::Pending,
            access_modes: None,
            capacity: None,
            conditions: None,
        }),
    };

    let pvc_key = build_key("persistentvolumeclaims", Some("default"), "capacity-test-pvc");
    storage.create(&pvc_key, &pvc).await.unwrap();

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify PV has capacity 10Gi
    let pv_name = "pvc-default-capacity-test-pvc";
    let pv_key = build_key("persistentvolumes", None, pv_name);
    let pv: PersistentVolume = storage.get(&pv_key).await.unwrap();

    assert_eq!(pv.spec.capacity.get("storage"), Some(&"10Gi".to_string()));
}

#[tokio::test]
async fn test_honors_access_modes() {
    let storage = setup_test().await;

    // Create StorageClass
    let sc = StorageClass {
        type_meta: TypeMeta {
            kind: "StorageClass".to_string(),
            api_version: "storage.k8s.io/v1".to_string(),
        },
        metadata: ObjectMeta::new("fast"),
        provisioner: "rusternetes.io/hostpath".to_string(),
        parameters: None,
        reclaim_policy: Some(PersistentVolumeReclaimPolicy::Delete),
        volume_binding_mode: Some(VolumeBindingMode::Immediate),
        allowed_topologies: None,
        allow_volume_expansion: None,
    };

    let sc_key = build_key("storageclasses", None, "fast");
    storage.create(&sc_key, &sc).await.unwrap();

    // Create PVC with ReadWriteMany
    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "5Gi".to_string());

    let pvc = PersistentVolumeClaim {
        type_meta: TypeMeta {
            kind: "PersistentVolumeClaim".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("rwm-pvc");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: vec![
                PersistentVolumeAccessMode::ReadWriteMany,
                PersistentVolumeAccessMode::ReadOnlyMany,
            ],
            resources: ResourceRequirements {
                limits: None,
                requests: Some(requests),
            },
            volume_name: None,
            storage_class_name: Some("fast".to_string()),
            volume_mode: Some(PersistentVolumeMode::Filesystem),
            selector: None,
            data_source: None,
        },
        status: Some(PersistentVolumeClaimStatus {
            phase: PersistentVolumeClaimPhase::Pending,
            access_modes: None,
            capacity: None,
            conditions: None,
        }),
    };

    let pvc_key = build_key("persistentvolumeclaims", Some("default"), "rwm-pvc");
    storage.create(&pvc_key, &pvc).await.unwrap();

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify PV has same access modes
    let pv_name = "pvc-default-rwm-pvc";
    let pv_key = build_key("persistentvolumes", None, pv_name);
    let pv: PersistentVolume = storage.get(&pv_key).await.unwrap();

    assert_eq!(pv.spec.access_modes, vec![
        PersistentVolumeAccessMode::ReadWriteMany,
        PersistentVolumeAccessMode::ReadOnlyMany,
    ]);
}

#[tokio::test]
async fn test_honors_reclaim_policy_delete() {
    let storage = setup_test().await;

    // Create StorageClass with Delete reclaim policy
    let sc = StorageClass {
        type_meta: TypeMeta {
            kind: "StorageClass".to_string(),
            api_version: "storage.k8s.io/v1".to_string(),
        },
        metadata: ObjectMeta::new("delete-policy"),
        provisioner: "rusternetes.io/hostpath".to_string(),
        parameters: None,
        reclaim_policy: Some(PersistentVolumeReclaimPolicy::Delete),
        volume_binding_mode: Some(VolumeBindingMode::Immediate),
        allowed_topologies: None,
        allow_volume_expansion: None,
    };

    let sc_key = build_key("storageclasses", None, "delete-policy");
    storage.create(&sc_key, &sc).await.unwrap();

    // Create PVC
    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "5Gi".to_string());

    let pvc = PersistentVolumeClaim {
        type_meta: TypeMeta {
            kind: "PersistentVolumeClaim".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("delete-pvc");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
            resources: ResourceRequirements {
                limits: None,
                requests: Some(requests),
            },
            volume_name: None,
            storage_class_name: Some("delete-policy".to_string()),
            volume_mode: Some(PersistentVolumeMode::Filesystem),
            selector: None,
            data_source: None,
        },
        status: Some(PersistentVolumeClaimStatus {
            phase: PersistentVolumeClaimPhase::Pending,
            access_modes: None,
            capacity: None,
            conditions: None,
        }),
    };

    let pvc_key = build_key("persistentvolumeclaims", Some("default"), "delete-pvc");
    storage.create(&pvc_key, &pvc).await.unwrap();

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify PV has Delete reclaim policy
    let pv_name = "pvc-default-delete-pvc";
    let pv_key = build_key("persistentvolumes", None, pv_name);
    let pv: PersistentVolume = storage.get(&pv_key).await.unwrap();

    assert_eq!(pv.spec.persistent_volume_reclaim_policy, Some(PersistentVolumeReclaimPolicy::Delete));
}

#[tokio::test]
async fn test_honors_reclaim_policy_retain() {
    let storage = setup_test().await;

    // Create StorageClass with Retain reclaim policy
    let sc = StorageClass {
        type_meta: TypeMeta {
            kind: "StorageClass".to_string(),
            api_version: "storage.k8s.io/v1".to_string(),
        },
        metadata: ObjectMeta::new("retain-policy"),
        provisioner: "rusternetes.io/hostpath".to_string(),
        parameters: None,
        reclaim_policy: Some(PersistentVolumeReclaimPolicy::Retain),
        volume_binding_mode: Some(VolumeBindingMode::Immediate),
        allowed_topologies: None,
        allow_volume_expansion: None,
    };

    let sc_key = build_key("storageclasses", None, "retain-policy");
    storage.create(&sc_key, &sc).await.unwrap();

    // Create PVC
    let mut requests = HashMap::new();
    requests.insert("storage".to_string(), "5Gi".to_string());

    let pvc = PersistentVolumeClaim {
        type_meta: TypeMeta {
            kind: "PersistentVolumeClaim".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new("retain-pvc");
            meta.namespace = Some("default".to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: PersistentVolumeClaimSpec {
            access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
            resources: ResourceRequirements {
                limits: None,
                requests: Some(requests),
            },
            volume_name: None,
            storage_class_name: Some("retain-policy".to_string()),
            volume_mode: Some(PersistentVolumeMode::Filesystem),
            selector: None,
            data_source: None,
        },
        status: Some(PersistentVolumeClaimStatus {
            phase: PersistentVolumeClaimPhase::Pending,
            access_modes: None,
            capacity: None,
            conditions: None,
        }),
    };

    let pvc_key = build_key("persistentvolumeclaims", Some("default"), "retain-pvc");
    storage.create(&pvc_key, &pvc).await.unwrap();

    // Run provisioner
    let controller = DynamicProvisionerController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify PV has Retain reclaim policy
    let pv_name = "pvc-default-retain-pvc";
    let pv_key = build_key("persistentvolumes", None, pv_name);
    let pv: PersistentVolume = storage.get(&pv_key).await.unwrap();

    assert_eq!(pv.spec.persistent_volume_reclaim_policy, Some(PersistentVolumeReclaimPolicy::Retain));
}

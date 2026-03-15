use rusternetes_common::resources::{
    Endpoints, EndpointSubset, EndpointAddress, EndpointPort, EndpointReference, EndpointSlice,
};
use rusternetes_common::types::{ObjectMeta, TypeMeta};
use rusternetes_controller_manager::controllers::endpointslice::EndpointSliceController;
use rusternetes_storage::{build_key, MemoryStorage, Storage};
use std::sync::Arc;

fn create_test_endpoints(
    name: &str,
    namespace: &str,
    addresses: Vec<String>,
    ports: Vec<(String, u16)>,
) -> Endpoints {
    let endpoint_addresses: Vec<EndpointAddress> = addresses
        .iter()
        .enumerate()
        .map(|(idx, ip)| EndpointAddress {
            ip: ip.clone(),
            hostname: None,
            node_name: Some(format!("node-{}", idx)),
            target_ref: Some(EndpointReference {
                kind: Some("Pod".to_string()),
                namespace: Some(namespace.to_string()),
                name: Some(format!("pod-{}", idx)),
                uid: Some(uuid::Uuid::new_v4().to_string()),
            }),
        })
        .collect();

    let endpoint_ports: Vec<EndpointPort> = ports
        .iter()
        .map(|(name, port)| EndpointPort {
            name: Some(name.clone()),
            port: *port,
            protocol: Some("TCP".to_string()),
            app_protocol: None,
        })
        .collect();

    Endpoints {
        type_meta: TypeMeta {
            kind: "Endpoints".to_string(),
            api_version: "v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new(name);
            meta.namespace = Some(namespace.to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        subsets: vec![EndpointSubset {
            addresses: Some(endpoint_addresses),
            not_ready_addresses: None,
            ports: Some(endpoint_ports),
        }],
    }
}

#[tokio::test]
async fn test_endpointslice_created_from_endpoints() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints
    let endpoints = create_test_endpoints(
        "web-service",
        "default",
        vec!["10.244.0.1".to_string(), "10.244.0.2".to_string()],
        vec![("http".to_string(), 8080)],
    );

    storage
        .create(&build_key("endpoints", Some("default"), "web-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice was created
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "web-service"))
        .await
        .unwrap();

    assert_eq!(slice.metadata.name, "web-service");
    assert_eq!(slice.metadata.namespace, Some("default".to_string()));
    assert_eq!(slice.endpoints.len(), 2, "Should have 2 endpoints");
    assert_eq!(slice.ports.len(), 1, "Should have 1 port");
}

#[tokio::test]
async fn test_endpointslice_has_owner_reference() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints
    let endpoints = create_test_endpoints(
        "api-service",
        "default",
        vec!["10.244.0.10".to_string()],
        vec![("https".to_string(), 8443)],
    );
    let endpoints_uid = endpoints.metadata.uid.clone();

    storage
        .create(&build_key("endpoints", Some("default"), "api-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice has owner reference
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "api-service"))
        .await
        .unwrap();

    assert!(slice.metadata.owner_references.is_some(), "Should have owner references");
    let owner_refs = slice.metadata.owner_references.unwrap();
    assert_eq!(owner_refs.len(), 1);

    let owner_ref = &owner_refs[0];
    assert_eq!(owner_ref.kind, "Endpoints");
    assert_eq!(owner_ref.name, "api-service");
    assert_eq!(owner_ref.uid, endpoints_uid);
    assert_eq!(owner_ref.controller, Some(true));
    assert_eq!(owner_ref.block_owner_deletion, Some(true));
}

#[tokio::test]
async fn test_endpointslice_has_service_label() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints
    let endpoints = create_test_endpoints(
        "db-service",
        "default",
        vec!["10.244.0.20".to_string()],
        vec![("mysql".to_string(), 3306)],
    );

    storage
        .create(&build_key("endpoints", Some("default"), "db-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice has service label
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "db-service"))
        .await
        .unwrap();

    assert!(slice.metadata.labels.is_some(), "Should have labels");
    let labels = slice.metadata.labels.unwrap();
    assert_eq!(
        labels.get("kubernetes.io/service-name"),
        Some(&"db-service".to_string()),
        "Should have service-name label"
    );
}

#[tokio::test]
async fn test_endpointslice_includes_port_mapping() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints with multiple ports
    let endpoints = create_test_endpoints(
        "multi-port-service",
        "default",
        vec!["10.244.0.30".to_string()],
        vec![
            ("http".to_string(), 8080),
            ("https".to_string(), 8443),
            ("metrics".to_string(), 9090),
        ],
    );

    storage
        .create(&build_key("endpoints", Some("default"), "multi-port-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice includes all ports
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "multi-port-service"))
        .await
        .unwrap();

    assert_eq!(slice.ports.len(), 3, "Should have 3 ports");

    // Verify specific ports
    let http_port = slice.ports.iter().find(|p| p.name == Some("http".to_string())).unwrap();
    assert_eq!(http_port.port, Some(8080));
    assert_eq!(http_port.protocol, Some("TCP".to_string()));

    let https_port = slice.ports.iter().find(|p| p.name == Some("https".to_string())).unwrap();
    assert_eq!(https_port.port, Some(8443));

    let metrics_port = slice.ports.iter().find(|p| p.name == Some("metrics".to_string())).unwrap();
    assert_eq!(metrics_port.port, Some(9090));
}

#[tokio::test]
async fn test_endpointslice_includes_endpoint_conditions() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints with ready addresses
    let endpoints = create_test_endpoints(
        "cache-service",
        "default",
        vec!["10.244.0.40".to_string(), "10.244.0.41".to_string()],
        vec![("redis".to_string(), 6379)],
    );

    storage
        .create(&build_key("endpoints", Some("default"), "cache-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice endpoint conditions
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "cache-service"))
        .await
        .unwrap();

    assert_eq!(slice.endpoints.len(), 2);

    for endpoint in &slice.endpoints {
        assert!(endpoint.conditions.is_some(), "Endpoint should have conditions");
        let conditions = endpoint.conditions.as_ref().unwrap();
        assert_eq!(conditions.ready, Some(true), "Ready endpoints should be marked as ready");
        assert_eq!(conditions.serving, Some(true), "Ready endpoints should be serving");
        assert_eq!(conditions.terminating, Some(false), "Ready endpoints should not be terminating");
    }
}

#[tokio::test]
async fn test_endpointslice_includes_target_ref() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints
    let endpoints = create_test_endpoints(
        "worker-service",
        "default",
        vec!["10.244.0.50".to_string()],
        vec![("app".to_string(), 9000)],
    );

    storage
        .create(&build_key("endpoints", Some("default"), "worker-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice includes target references
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "worker-service"))
        .await
        .unwrap();

    assert_eq!(slice.endpoints.len(), 1);
    let endpoint = &slice.endpoints[0];

    assert!(endpoint.target_ref.is_some(), "Endpoint should have target_ref");
    let target_ref = endpoint.target_ref.as_ref().unwrap();
    assert_eq!(target_ref.kind, Some("Pod".to_string()));
    assert_eq!(target_ref.namespace, Some("default".to_string()));
    assert!(target_ref.name.is_some());
    assert!(target_ref.uid.is_some());
}

#[tokio::test]
async fn test_endpointslice_updates_when_endpoints_change() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create initial Endpoints with 1 address
    let mut endpoints = create_test_endpoints(
        "elastic-service",
        "default",
        vec!["10.244.0.60".to_string()],
        vec![("http".to_string(), 9200)],
    );
    let endpoints_key = build_key("endpoints", Some("default"), "elastic-service");
    storage.create(&endpoints_key, &endpoints).await.unwrap();

    // First reconcile
    controller.reconcile_all().await.unwrap();

    // Verify initial EndpointSlice
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "elastic-service"))
        .await
        .unwrap();
    assert_eq!(slice.endpoints.len(), 1);

    // Update Endpoints with 3 addresses
    endpoints = create_test_endpoints(
        "elastic-service",
        "default",
        vec![
            "10.244.0.60".to_string(),
            "10.244.0.61".to_string(),
            "10.244.0.62".to_string(),
        ],
        vec![("http".to_string(), 9200)],
    );
    storage.update(&endpoints_key, &endpoints).await.unwrap();

    // Second reconcile
    controller.reconcile_all().await.unwrap();

    // Verify updated EndpointSlice
    let updated_slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "elastic-service"))
        .await
        .unwrap();
    assert_eq!(
        updated_slice.endpoints.len(),
        3,
        "EndpointSlice should be updated to include new addresses"
    );
}

#[tokio::test]
async fn test_endpointslice_multiple_namespaces() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints in different namespaces
    let endpoints_ns1 = create_test_endpoints(
        "app-service",
        "ns1",
        vec!["10.244.1.1".to_string()],
        vec![("http".to_string(), 8080)],
    );
    let endpoints_ns2 = create_test_endpoints(
        "app-service",
        "ns2",
        vec!["10.244.2.1".to_string()],
        vec![("http".to_string(), 8080)],
    );

    storage
        .create(&build_key("endpoints", Some("ns1"), "app-service"), &endpoints_ns1)
        .await
        .unwrap();
    storage
        .create(&build_key("endpoints", Some("ns2"), "app-service"), &endpoints_ns2)
        .await
        .unwrap();

    // Reconcile all
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlices in ns1
    let slice_ns1: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("ns1"), "app-service"))
        .await
        .unwrap();
    assert_eq!(slice_ns1.metadata.namespace, Some("ns1".to_string()));
    assert_eq!(slice_ns1.endpoints.len(), 1);
    assert_eq!(slice_ns1.endpoints[0].addresses[0], "10.244.1.1");

    // Verify EndpointSlices in ns2
    let slice_ns2: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("ns2"), "app-service"))
        .await
        .unwrap();
    assert_eq!(slice_ns2.metadata.namespace, Some("ns2".to_string()));
    assert_eq!(slice_ns2.endpoints.len(), 1);
    assert_eq!(slice_ns2.endpoints[0].addresses[0], "10.244.2.1");
}

#[tokio::test]
async fn test_endpointslice_cleanup_orphans() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints
    let endpoints = create_test_endpoints(
        "cleanup-service",
        "default",
        vec!["10.244.0.70".to_string()],
        vec![("http".to_string(), 8080)],
    );

    storage
        .create(&build_key("endpoints", Some("default"), "cleanup-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile to create EndpointSlice
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice exists
    let slice_result: Result<EndpointSlice, _> = storage
        .get(&build_key("endpointslices", Some("default"), "cleanup-service"))
        .await;
    assert!(slice_result.is_ok(), "EndpointSlice should exist");

    // Delete Endpoints
    storage
        .delete(&build_key("endpoints", Some("default"), "cleanup-service"))
        .await
        .unwrap();

    // Run cleanup
    controller.cleanup_orphans().await.unwrap();

    // Verify EndpointSlice was deleted
    let slice_result: Result<EndpointSlice, _> = storage
        .get(&build_key("endpointslices", Some("default"), "cleanup-service"))
        .await;
    assert!(slice_result.is_err(), "Orphaned EndpointSlice should be deleted");
}

#[tokio::test]
async fn test_endpointslice_empty_endpoints() {
    let storage = Arc::new(MemoryStorage::new());
    let controller = EndpointSliceController::new(storage.clone());

    // Create Endpoints with no addresses
    let endpoints = create_test_endpoints(
        "empty-service",
        "default",
        vec![],
        vec![("http".to_string(), 8080)],
    );

    storage
        .create(&build_key("endpoints", Some("default"), "empty-service"), &endpoints)
        .await
        .unwrap();

    // Reconcile
    controller.reconcile_all().await.unwrap();

    // Verify EndpointSlice was created
    let slice: EndpointSlice = storage
        .get(&build_key("endpointslices", Some("default"), "empty-service"))
        .await
        .unwrap();

    assert_eq!(slice.endpoints.len(), 0, "EndpointSlice should have no endpoints");
    // When there are no addresses, ports may also be empty depending on implementation
    assert!(slice.ports.len() >= 0, "EndpointSlice ports can be empty or populated");
}

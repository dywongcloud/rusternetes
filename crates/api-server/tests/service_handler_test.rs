//! Integration tests for Service handler
//!
//! Tests all CRUD operations, edge cases, and error handling for services

use rusternetes_common::resources::{Service, ServicePort, ServiceSpec, ServiceType};
use rusternetes_common::types::Metadata;
use rusternetes_storage::{build_key, build_prefix, memory::MemoryStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;

/// Helper function to create a test service
fn create_test_service(name: &str, namespace: &str, service_type: ServiceType) -> Service {
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), name.to_string());

    let mut selector = HashMap::new();
    selector.insert("app".to_string(), name.to_string());

    Service {
        metadata: Metadata {
            name: name.to_string(),
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
        },
        spec: ServiceSpec {
            selector: Some(selector),
            ports: Some(vec![ServicePort {
                name: Some("http".to_string()),
                protocol: Some("TCP".to_string()),
                port: 80,
                target_port: Some(rusternetes_common::types::IntOrString::Int(8080)),
                node_port: None,
                app_protocol: None,
            }]),
            cluster_ip: None,
            cluster_ips: None,
            service_type: Some(service_type),
            external_ips: None,
            session_affinity: Some("None".to_string()),
            load_balancer_ip: None,
            load_balancer_source_ranges: None,
            external_name: None,
            external_traffic_policy: None,
            health_check_node_port: None,
            publish_not_ready_addresses: None,
            session_affinity_config: None,
            ip_families: None,
            ip_family_policy: None,
            allocate_load_balancer_node_ports: None,
            load_balancer_class: None,
            internal_traffic_policy: None,
        },
        status: None,
    }
}

#[tokio::test]
async fn test_service_create_and_get() {
    let storage = Arc::new(MemoryStorage::new());

    let service = create_test_service("test-svc", "default", ServiceType::ClusterIP);
    let key = build_key("services", Some("default"), "test-svc");

    // Create
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(created.metadata.name, "test-svc");
    assert_eq!(created.metadata.namespace, Some("default".to_string()));
    assert_eq!(
        created.spec.service_type,
        Some(ServiceType::ClusterIP)
    );
    assert!(created.metadata.uid.is_some());

    // Get
    let retrieved: Service = storage.get(&key).await.unwrap();
    assert_eq!(retrieved.metadata.name, "test-svc");
    assert_eq!(
        retrieved.spec.service_type,
        Some(ServiceType::ClusterIP)
    );

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_update() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-update-svc", "default", ServiceType::ClusterIP);
    let key = build_key("services", Some("default"), "test-update-svc");

    // Create
    storage.create(&key, &service).await.unwrap();

    // Update service type to NodePort
    service.spec.service_type = Some(ServiceType::NodePort);
    let updated: Service = storage.update(&key, &service).await.unwrap();
    assert_eq!(updated.spec.service_type, Some(ServiceType::NodePort));

    // Verify update
    let retrieved: Service = storage.get(&key).await.unwrap();
    assert_eq!(retrieved.spec.service_type, Some(ServiceType::NodePort));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_delete() {
    let storage = Arc::new(MemoryStorage::new());

    let service = create_test_service("test-delete-svc", "default", ServiceType::ClusterIP);
    let key = build_key("services", Some("default"), "test-delete-svc");

    // Create
    storage.create(&key, &service).await.unwrap();

    // Delete
    storage.delete(&key).await.unwrap();

    // Verify deletion
    let result = storage.get::<Service>(&key).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_service_list() {
    let storage = Arc::new(MemoryStorage::new());

    // Create multiple services
    let svc1 = create_test_service("svc-1", "default", ServiceType::ClusterIP);
    let svc2 = create_test_service("svc-2", "default", ServiceType::NodePort);
    let svc3 = create_test_service("svc-3", "default", ServiceType::LoadBalancer);

    let key1 = build_key("services", Some("default"), "svc-1");
    let key2 = build_key("services", Some("default"), "svc-2");
    let key3 = build_key("services", Some("default"), "svc-3");

    storage.create(&key1, &svc1).await.unwrap();
    storage.create(&key2, &svc2).await.unwrap();
    storage.create(&key3, &svc3).await.unwrap();

    // List
    let prefix = build_prefix("services", Some("default"));
    let services: Vec<Service> = storage.list(&prefix).await.unwrap();

    assert!(services.len() >= 3);
    let names: Vec<String> = services.iter().map(|s| s.metadata.name.clone()).collect();
    assert!(names.contains(&"svc-1".to_string()));
    assert!(names.contains(&"svc-2".to_string()));
    assert!(names.contains(&"svc-3".to_string()));

    // Clean up
    storage.delete(&key1).await.unwrap();
    storage.delete(&key2).await.unwrap();
    storage.delete(&key3).await.unwrap();
}

#[tokio::test]
async fn test_service_list_across_namespaces() {
    let storage = Arc::new(MemoryStorage::new());

    // Create services in different namespaces
    let svc1 = create_test_service("svc-ns1", "namespace-1", ServiceType::ClusterIP);
    let svc2 = create_test_service("svc-ns2", "namespace-2", ServiceType::ClusterIP);

    let key1 = build_key("services", Some("namespace-1"), "svc-ns1");
    let key2 = build_key("services", Some("namespace-2"), "svc-ns2");

    storage.create(&key1, &svc1).await.unwrap();
    storage.create(&key2, &svc2).await.unwrap();

    // List all (no namespace filter)
    let prefix = build_prefix("services", None);
    let services: Vec<Service> = storage.list(&prefix).await.unwrap();

    // Should find at least our 2 services
    assert!(services.len() >= 2);

    // Clean up
    storage.delete(&key1).await.unwrap();
    storage.delete(&key2).await.unwrap();
}

#[tokio::test]
async fn test_service_with_nodeport() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-nodeport", "default", ServiceType::NodePort);

    // Set NodePort
    if let Some(ports) = &mut service.spec.ports {
        ports[0].node_port = Some(30080);
    }

    let key = build_key("services", Some("default"), "test-nodeport");

    // Create with NodePort
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(created.spec.service_type, Some(ServiceType::NodePort));
    assert_eq!(created.spec.ports.as_ref().unwrap()[0].node_port, Some(30080));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_with_loadbalancer() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-lb", "default", ServiceType::LoadBalancer);
    service.spec.load_balancer_ip = Some("1.2.3.4".to_string());

    let key = build_key("services", Some("default"), "test-lb");

    // Create with LoadBalancer
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(created.spec.service_type, Some(ServiceType::LoadBalancer));
    assert_eq!(created.spec.load_balancer_ip, Some("1.2.3.4".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_with_externalname() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-extname", "default", ServiceType::ExternalName);
    service.spec.external_name = Some("example.com".to_string());
    service.spec.cluster_ip = Some("None".to_string());

    let key = build_key("services", Some("default"), "test-extname");

    // Create with ExternalName
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(created.spec.service_type, Some(ServiceType::ExternalName));
    assert_eq!(created.spec.external_name, Some("example.com".to_string()));
    assert_eq!(created.spec.cluster_ip, Some("None".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_with_finalizers() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-finalizers", "default", ServiceType::ClusterIP);
    service.metadata.finalizers = Some(vec!["service.finalizer.io".to_string()]);

    let key = build_key("services", Some("default"), "test-finalizers");

    // Create with finalizer
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(
        created.metadata.finalizers,
        Some(vec!["service.finalizer.io".to_string()])
    );

    // Verify finalizer is present
    let retrieved: Service = storage.get(&key).await.unwrap();
    assert_eq!(
        retrieved.metadata.finalizers,
        Some(vec!["service.finalizer.io".to_string()])
    );

    // Clean up - remove finalizer first
    service.metadata.finalizers = None;
    storage.update(&key, &service).await.unwrap();
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_metadata_immutability() {
    let storage = Arc::new(MemoryStorage::new());

    let service = create_test_service("test-immutable", "default", ServiceType::ClusterIP);
    let key = build_key("services", Some("default"), "test-immutable");

    // Create
    let created: Service = storage.create(&key, &service).await.unwrap();
    let original_uid = created.metadata.uid.clone();

    // Try to update - UID should remain unchanged
    let mut updated_svc = created.clone();
    updated_svc.spec.session_affinity = Some("ClientIP".to_string());

    let updated: Service = storage.update(&key, &updated_svc).await.unwrap();
    assert_eq!(updated.metadata.uid, original_uid);
    assert_eq!(updated.spec.session_affinity, Some("ClientIP".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_with_multiple_ports() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-multiport", "default", ServiceType::ClusterIP);
    service.spec.ports = Some(vec![
        ServicePort {
            name: Some("http".to_string()),
            protocol: Some("TCP".to_string()),
            port: 80,
            target_port: Some(rusternetes_common::types::IntOrString::Int(8080)),
            node_port: None,
            app_protocol: None,
        },
        ServicePort {
            name: Some("https".to_string()),
            protocol: Some("TCP".to_string()),
            port: 443,
            target_port: Some(rusternetes_common::types::IntOrString::Int(8443)),
            node_port: None,
            app_protocol: None,
        },
    ]);

    let key = build_key("services", Some("default"), "test-multiport");

    // Create with multiple ports
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(created.spec.ports.as_ref().unwrap().len(), 2);

    let ports = created.spec.ports.unwrap();
    assert_eq!(ports[0].port, 80);
    assert_eq!(ports[1].port, 443);

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_selector() {
    let storage = Arc::new(MemoryStorage::new());

    let mut selector = HashMap::new();
    selector.insert("app".to_string(), "backend".to_string());
    selector.insert("tier".to_string(), "api".to_string());

    let mut service = create_test_service("test-selector", "default", ServiceType::ClusterIP);
    service.spec.selector = Some(selector.clone());

    let key = build_key("services", Some("default"), "test-selector");

    // Create with selector
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert!(created.spec.selector.is_some());

    let created_selector = created.spec.selector.unwrap();
    assert_eq!(created_selector.get("app"), Some(&"backend".to_string()));
    assert_eq!(created_selector.get("tier"), Some(&"api".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_get_not_found() {
    let storage = Arc::new(MemoryStorage::new());

    let key = build_key("services", Some("default"), "nonexistent");
    let result = storage.get::<Service>(&key).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_service_update_not_found() {
    let storage = Arc::new(MemoryStorage::new());

    let service = create_test_service("nonexistent", "default", ServiceType::ClusterIP);
    let key = build_key("services", Some("default"), "nonexistent");

    let result = storage.update(&key, &service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_service_headless() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-headless", "default", ServiceType::ClusterIP);
    service.spec.cluster_ip = Some("None".to_string());

    let key = build_key("services", Some("default"), "test-headless");

    // Create headless service
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(created.spec.cluster_ip, Some("None".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

#[tokio::test]
async fn test_service_session_affinity() {
    let storage = Arc::new(MemoryStorage::new());

    let mut service = create_test_service("test-affinity", "default", ServiceType::ClusterIP);
    service.spec.session_affinity = Some("ClientIP".to_string());

    let key = build_key("services", Some("default"), "test-affinity");

    // Create with session affinity
    let created: Service = storage.create(&key, &service).await.unwrap();
    assert_eq!(created.spec.session_affinity, Some("ClientIP".to_string()));

    // Clean up
    storage.delete(&key).await.unwrap();
}

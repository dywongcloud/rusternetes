use hickory_proto::rr::{Name, RecordType};
use rusternetes_dns_server::resolver::{KubernetesResolver, ServiceEndpoint};
use std::str::FromStr;
use std::sync::Arc;

#[test]
fn test_service_clusterip_resolution() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a ClusterIP service
    resolver.update_service(
        "nginx",
        "default",
        Some("10.96.1.100"),
        vec![],
    );

    // Query the service
    let name = Name::from_str("nginx.default.svc.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
    let records = records.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_type(), RecordType::A);
}

#[test]
fn test_headless_service_resolution() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a headless service with multiple endpoints
    let endpoints = vec![
        ServiceEndpoint {
            ip: "10.244.1.5".to_string(),
            port: 80,
            port_name: Some("http".to_string()),
            protocol: Some("tcp".to_string()),
            pod_name: Some("nginx-pod-1".to_string()),
        },
        ServiceEndpoint {
            ip: "10.244.1.6".to_string(),
            port: 80,
            port_name: Some("http".to_string()),
            protocol: Some("tcp".to_string()),
            pod_name: Some("nginx-pod-2".to_string()),
        },
    ];

    resolver.update_service(
        "nginx-headless",
        "default",
        None, // No ClusterIP
        endpoints,
    );

    // Query the headless service
    let name = Name::from_str("nginx-headless.default.svc.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
    let records = records.unwrap();
    assert_eq!(records.len(), 2); // Two pod IPs
    assert!(records.iter().all(|r| r.record_type() == RecordType::A));
}

#[test]
fn test_srv_record_resolution() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a headless service with port information
    let endpoints = vec![
        ServiceEndpoint {
            ip: "10.244.1.5".to_string(),
            port: 80,
            port_name: Some("http".to_string()),
            protocol: Some("tcp".to_string()),
            pod_name: Some("nginx-pod-1".to_string()),
        },
    ];

    resolver.update_service(
        "nginx-headless",
        "default",
        None, // No ClusterIP
        endpoints,
    );

    // Query SRV record
    let name = Name::from_str("_http._tcp.nginx-headless.default.svc.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
    let records = records.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_type(), RecordType::SRV);
}

#[test]
fn test_pod_name_resolution() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a pod
    resolver.update_pod("nginx-pod-abc123", "default", "10.244.1.5");

    // Query the pod by name
    let name = Name::from_str("nginx-pod-abc123.default.pod.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
    let records = records.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_type(), RecordType::A);
}

#[test]
fn test_pod_ip_based_resolution() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a pod
    resolver.update_pod("nginx-pod-abc123", "default", "10.244.1.5");

    // Query the pod by IP (with dashes)
    let name = Name::from_str("10-244-1-5.default.pod.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
    let records = records.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_type(), RecordType::A);
}

#[test]
fn test_service_removal() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a service
    resolver.update_service("nginx", "default", Some("10.96.1.100"), vec![]);

    // Verify it exists
    let name = Name::from_str("nginx.default.svc.cluster.local").unwrap();
    assert!(resolver.lookup(&name).is_some());

    // Remove the service
    resolver.remove_service("nginx", "default");

    // Verify it's gone
    assert!(resolver.lookup(&name).is_none());
}

#[test]
fn test_pod_removal() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a pod
    resolver.update_pod("nginx-pod-abc123", "default", "10.244.1.5");

    // Verify it exists
    let name = Name::from_str("nginx-pod-abc123.default.pod.cluster.local").unwrap();
    assert!(resolver.lookup(&name).is_some());

    // Remove the pod
    resolver.remove_pod("nginx-pod-abc123", "default", Some("10.244.1.5"));

    // Verify it's gone
    assert!(resolver.lookup(&name).is_none());
}

#[test]
fn test_multiple_namespaces() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add services in different namespaces
    resolver.update_service("nginx", "default", Some("10.96.1.100"), vec![]);
    resolver.update_service("nginx", "production", Some("10.96.2.100"), vec![]);

    // Query default namespace
    let name_default = Name::from_str("nginx.default.svc.cluster.local").unwrap();
    let records_default = resolver.lookup(&name_default);
    assert!(records_default.is_some());

    // Query production namespace
    let name_prod = Name::from_str("nginx.production.svc.cluster.local").unwrap();
    let records_prod = resolver.lookup(&name_prod);
    assert!(records_prod.is_some());

    // Verify they're different
    assert_ne!(
        records_default.unwrap()[0].data(),
        records_prod.unwrap()[0].data()
    );
}

#[test]
fn test_nonexistent_lookup() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Query a service that doesn't exist
    let name = Name::from_str("nonexistent.default.svc.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_none());
}

#[test]
fn test_resolver_stats() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add some records
    resolver.update_service("nginx", "default", Some("10.96.1.100"), vec![]);
    resolver.update_pod("pod-1", "default", "10.244.1.5");
    resolver.update_pod("pod-2", "default", "10.244.1.6");

    // Get stats
    let (names, records) = resolver.stats();

    // Should have:
    // - 1 service name
    // - 2 pod names (name-based)
    // - 2 pod names (IP-based)
    assert_eq!(names, 5);
    assert_eq!(records, 5);
}

#[test]
fn test_custom_cluster_domain() {
    let resolver = Arc::new(KubernetesResolver::new("example.com".to_string(), 10));

    // Add a service
    resolver.update_service("nginx", "default", Some("10.96.1.100"), vec![]);

    // Query with custom domain
    let name = Name::from_str("nginx.default.svc.example.com").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
}

#[test]
fn test_ipv6_support() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a service with IPv6
    resolver.update_service("nginx", "default", Some("fd00::1"), vec![]);

    // Query the service
    let name = Name::from_str("nginx.default.svc.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
    let records = records.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_type(), RecordType::AAAA);
}

#[test]
fn test_update_service_overwrites() {
    let resolver = Arc::new(KubernetesResolver::new("cluster.local".to_string(), 10));

    // Add a service
    resolver.update_service("nginx", "default", Some("10.96.1.100"), vec![]);

    // Update with new IP
    resolver.update_service("nginx", "default", Some("10.96.1.200"), vec![]);

    // Query should return new IP
    let name = Name::from_str("nginx.default.svc.cluster.local").unwrap();
    let records = resolver.lookup(&name);

    assert!(records.is_some());
    let records = records.unwrap();
    assert_eq!(records.len(), 1);
}

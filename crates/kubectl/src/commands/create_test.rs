#[cfg(test)]
mod tests {
    use super::super::create::*;

    #[test]
    fn test_kind_extraction_from_yaml() {
        let yaml = r#"
apiVersion: v1
kind: Pod
metadata:
  name: test-pod
spec:
  containers:
  - name: nginx
    image: nginx:latest
"#;

        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let kind = value.get("kind").and_then(|k| k.as_str());

        assert_eq!(kind, Some("Pod"));
    }

    #[test]
    fn test_missing_kind_field() {
        let yaml = r#"
apiVersion: v1
metadata:
  name: test-pod
"#;

        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let kind = value.get("kind").and_then(|k| k.as_str());

        assert_eq!(kind, None);
    }

    #[test]
    fn test_pod_deserialization() {
        let yaml = r#"
apiVersion: v1
kind: Pod
metadata:
  name: test-pod
  namespace: default
spec:
  containers:
  - name: nginx
    image: nginx:latest
"#;

        use rusternetes_common::resources::Pod;
        let pod: Pod = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(pod.metadata.name, "test-pod");
        assert_eq!(pod.metadata.namespace, Some("default".to_string()));
    }

    #[test]
    fn test_service_deserialization() {
        let yaml = r#"
apiVersion: v1
kind: Service
metadata:
  name: test-service
  namespace: default
spec:
  selector:
    app: test
  ports:
  - port: 80
    targetPort: 80
"#;

        use rusternetes_common::resources::Service;
        let service: Service = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(service.metadata.name, "test-service");
        assert_eq!(service.spec.ports.len(), 1);
        assert_eq!(service.spec.ports[0].port, 80);
    }

    #[test]
    fn test_deployment_deserialization() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-deployment
  namespace: default
spec:
  replicas: 3
  selector:
    matchLabels:
      app: test
  template:
    metadata:
      labels:
        app: test
    spec:
      containers:
      - name: nginx
        image: nginx:latest
"#;

        use rusternetes_common::resources::Deployment;
        let deployment: Deployment = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(deployment.metadata.name, "test-deployment");
        assert_eq!(deployment.spec.replicas, 3);
    }

    #[test]
    fn test_namespace_deserialization() {
        let yaml = r#"
apiVersion: v1
kind: Namespace
metadata:
  name: test-namespace
"#;

        use rusternetes_common::resources::Namespace;
        let namespace: Namespace = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(namespace.metadata.name, "test-namespace");
    }

    #[test]
    fn test_node_deserialization() {
        let yaml = r#"
apiVersion: v1
kind: Node
metadata:
  name: test-node
spec:
  podCIDR: "10.244.0.0/24"
"#;

        use rusternetes_common::resources::Node;
        let node: Node = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(node.metadata.name, "test-node");
    }

    #[test]
    fn test_storageclass_deserialization() {
        let yaml = r#"
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast-storage
provisioner: rusternetes.io/local
parameters:
  type: ssd
  iops: "1000"
reclaimPolicy: Delete
volumeBindingMode: Immediate
allowVolumeExpansion: true
"#;

        use rusternetes_common::resources::StorageClass;
        use rusternetes_common::resources::volume::{PersistentVolumeReclaimPolicy, VolumeBindingMode};
        let sc: StorageClass = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(sc.metadata.name, "fast-storage");
        assert_eq!(sc.provisioner, "rusternetes.io/local");
        assert!(matches!(sc.reclaim_policy, Some(PersistentVolumeReclaimPolicy::Delete)));
        assert!(matches!(sc.volume_binding_mode, Some(VolumeBindingMode::Immediate)));
        assert_eq!(sc.allow_volume_expansion, Some(true));

        // Check parameters
        let params = sc.parameters.as_ref().unwrap();
        assert!(params.contains_key("type"));
        assert_eq!(params.get("type").map(|s| s.as_str()), Some("ssd"));
        assert_eq!(params.get("iops").map(|s| s.as_str()), Some("1000"));
    }

    #[test]
    fn test_all_supported_resource_kinds() {
        let test_cases = vec![
            ("Pod", "v1"),
            ("Service", "v1"),
            ("Deployment", "apps/v1"),
            ("Node", "v1"),
            ("Namespace", "v1"),
            ("StorageClass", "storage.k8s.io/v1"),
        ];

        for (kind, api_version) in test_cases {
            let yaml = format!(
                r#"
apiVersion: {}
kind: {}
metadata:
  name: test-resource
"#,
                api_version, kind
            );

            let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
            let extracted_kind = value.get("kind").and_then(|k| k.as_str());
            let extracted_api = value.get("apiVersion").and_then(|k| k.as_str());

            assert_eq!(extracted_kind, Some(kind), "Kind should match for {}", kind);
            assert_eq!(extracted_api, Some(api_version), "API version should match for {}", kind);
        }
    }

    #[test]
    fn test_storageclass_with_minimal_fields() {
        let yaml = r#"
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: simple-storage
provisioner: rusternetes.io/simple
"#;

        use rusternetes_common::resources::StorageClass;
        let sc: StorageClass = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(sc.metadata.name, "simple-storage");
        assert_eq!(sc.provisioner, "rusternetes.io/simple");
        assert!(sc.parameters.is_none() || sc.parameters.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_storageclass_with_multiple_parameters() {
        let yaml = r#"
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: custom-storage
provisioner: rusternetes.io/custom
parameters:
  type: ssd
  iops: "1000"
  replication: "3"
  zone: "us-east-1a"
"#;

        use rusternetes_common::resources::StorageClass;
        let sc: StorageClass = serde_yaml::from_str(yaml).unwrap();

        let params = sc.parameters.as_ref().unwrap();
        assert_eq!(params.len(), 4);
        assert_eq!(params.get("type").map(|s| s.as_str()), Some("ssd"));
        assert_eq!(params.get("iops").map(|s| s.as_str()), Some("1000"));
        assert_eq!(params.get("replication").map(|s| s.as_str()), Some("3"));
        assert_eq!(params.get("zone").map(|s| s.as_str()), Some("us-east-1a"));
    }
}

// Integration tests for Job Controller

use rusternetes_common::resources::pod::*;
use rusternetes_common::resources::*;
use rusternetes_common::resources::workloads::*;
use rusternetes_common::types::{ObjectMeta, Phase, TypeMeta};
use rusternetes_controller_manager::controllers::job::JobController;
use rusternetes_storage::{build_key, Storage};
use rusternetes_storage::etcd::EtcdStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn setup_test() -> Arc<EtcdStorage> {
    let endpoints = vec!["http://localhost:2379".to_string()];
    Arc::new(EtcdStorage::new(endpoints).await.expect("Failed to create EtcdStorage"))
}

fn create_test_job(name: &str, namespace: &str, completions: i32, parallelism: i32) -> Job {
    let mut labels = HashMap::new();
    labels.insert("job-name".to_string(), name.to_string());

    Job {
        type_meta: TypeMeta {
            kind: "Job".to_string(),
            api_version: "batch/v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new(name);
            meta.namespace = Some(namespace.to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: JobSpec {
            completions: Some(completions),
            parallelism: Some(parallelism),
            backoff_limit: Some(3),
            template: PodTemplateSpec {
                metadata: Some({
                    let mut meta = ObjectMeta::new(&format!("{}-pod", name));
                    meta.labels = Some(labels);
                    meta
                }),
                spec: PodSpec {
                    containers: vec![Container {
                        name: "task".to_string(),
                        image: "busybox:latest".to_string(),
                        image_pull_policy: Some("IfNotPresent".to_string()),
                        command: Some(vec!["sh".to_string(), "-c".to_string(), "echo Hello".to_string()]),
                        ports: Some(vec![]),
                        env: None,
                        volume_mounts: None,
                        liveness_probe: None,
                        readiness_probe: None,
                        startup_probe: None,
                        resources: None,
                        working_dir: None,
                        args: None,
                        security_context: None,
                    }],
                    init_containers: None,
                    restart_policy: Some("Never".to_string()),
                    node_selector: None,
                    node_name: None,
                    volumes: None,
                    affinity: None,
                    tolerations: None,
                    service_account_name: None,
                    priority: None,
                    priority_class_name: None,
                    hostname: None,
                    host_network: None,
                    host_pid: None,
                    host_ipc: None,
                },
            },
            active_deadline_seconds: None,
        },
        status: None,
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_job_creates_pods() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/jobs/test-ns/task").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create job with 3 completions, parallelism 2
    let job = create_test_job("task", "test-ns", 3, 2);
    let key = build_key("jobs", Some("test-ns"), "task");
    storage.create(&key, &job).await.unwrap();

    // Run controller
    let controller = JobController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify 2 pods created (parallelism limit)
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert_eq!(pods.len(), 2, "Should create 2 pods initially (parallelism limit)");

    // Verify pods have correct labels
    for pod in &pods {
        let labels = pod.metadata.labels.as_ref().expect("Pod should have labels");
        assert_eq!(labels.get("job-name"), Some(&"task".to_string()));
    }

    // Cleanup
    let _ = storage.delete(&key).await;
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_job_respects_parallelism() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/jobs/test-ns/parallel").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create job with 10 completions, parallelism 3
    let job = create_test_job("parallel", "test-ns", 10, 3);
    let key = build_key("jobs", Some("test-ns"), "parallel");
    storage.create(&key, &job).await.unwrap();

    // Run controller
    let controller = JobController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify at most 3 pods created
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    assert!(pods.len() <= 3, "Should respect parallelism limit of 3");

    // Cleanup
    let _ = storage.delete(&key).await;
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_job_completion_detection() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/jobs/test-ns/complete").await;
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap_or_default();
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }

    // Create job with 2 completions
    let job = create_test_job("complete", "test-ns", 2, 2);
    let key = build_key("jobs", Some("test-ns"), "complete");
    storage.create(&key, &job).await.unwrap();

    // Run controller to create pods
    let controller = JobController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Mark both pods as succeeded
    let pods: Vec<Pod> = storage.list("/registry/pods/test-ns/").await.unwrap();
    for pod in &pods {
        let mut updated_pod = pod.clone();
        updated_pod.status = Some(PodStatus {
            phase: Phase::Succeeded,
            message: None,
            reason: None,
            pod_ip: None,
            host_ip: None,
            container_statuses: None,
            init_container_statuses: None,
        });
        let pod_key = build_key("pods", Some("test-ns"), &pod.metadata.name);
        storage.update(&pod_key, &updated_pod).await.unwrap();
    }

    // Run controller again
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify job is marked as complete
    let job: Job = storage.get(&key).await.unwrap();
    assert!(job.status.is_some());
    let status = job.status.unwrap();
    assert_eq!(status.succeeded, Some(2));

    if let Some(conditions) = status.conditions {
        assert!(conditions.iter().any(|c| c.condition_type == "Complete" && c.status == "True"));
    }

    // Cleanup
    let _ = storage.delete(&key).await;
    for pod in pods {
        let _ = storage.delete(&build_key("pods", Some("test-ns"), &pod.metadata.name)).await;
    }
}

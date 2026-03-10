// Integration tests for CronJob Controller

use rusternetes_common::resources::workloads::*;
use rusternetes_common::resources::PodTemplateSpec;
use rusternetes_common::resources::pod::*;
use rusternetes_common::types::{ObjectMeta, TypeMeta};
use rusternetes_controller_manager::controllers::cronjob::CronJobController;
use rusternetes_storage::{build_key, Storage};
use rusternetes_storage::etcd::EtcdStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn setup_test() -> Arc<EtcdStorage> {
    let endpoints = vec!["http://localhost:2379".to_string()];
    Arc::new(EtcdStorage::new(endpoints).await.expect("Failed to create EtcdStorage"))
}

fn create_test_cronjob(name: &str, namespace: &str, schedule: &str) -> CronJob {
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), name.to_string());

    CronJob {
        type_meta: TypeMeta {
            kind: "CronJob".to_string(),
            api_version: "batch/v1".to_string(),
        },
        metadata: {
            let mut meta = ObjectMeta::new(name);
            meta.namespace = Some(namespace.to_string());
            meta.uid = uuid::Uuid::new_v4().to_string();
            meta
        },
        spec: CronJobSpec {
            schedule: schedule.to_string(),
            job_template: JobTemplateSpec {
                metadata: Some({
                    let mut meta = ObjectMeta::new("");
                    meta.labels = Some(labels.clone());
                    meta
                }),
                spec: JobSpec {
                    completions: Some(1),
                    parallelism: Some(1),
                    backoff_limit: Some(3),
                    template: PodTemplateSpec {
                        metadata: Some({
                            let mut meta = ObjectMeta::new("");
                            meta.labels = Some(labels);
                            meta
                        }),
                        spec: PodSpec {
                            containers: vec![Container {
                                name: "task".to_string(),
                                image: "busybox:latest".to_string(),
                                image_pull_policy: Some("IfNotPresent".to_string()),
                                command: Some(vec!["echo".to_string(), "Hello".to_string()]),
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
            },
            suspend: Some(false),
            concurrency_policy: Some("Allow".to_string()),
            successful_jobs_history_limit: Some(3),
            failed_jobs_history_limit: Some(1),
        },
        status: None,
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_cronjob_creates_job() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/cronjobs/test-ns/hourly-backup").await;
    let jobs: Vec<Job> = storage.list("/registry/jobs/test-ns/").await.unwrap_or_default();
    for job in jobs {
        let _ = storage.delete(&build_key("jobs", Some("test-ns"), &job.metadata.name)).await;
    }

    // Create CronJob with @hourly schedule (will trigger immediately since never run)
    let cronjob = create_test_cronjob("hourly-backup", "test-ns", "@hourly");
    let key = build_key("cronjobs", Some("test-ns"), "hourly-backup");
    storage.create(&key, &cronjob).await.unwrap();

    // Run controller
    let controller = CronJobController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify job was created
    let jobs: Vec<Job> = storage.list("/registry/jobs/test-ns/").await.unwrap();
    assert_eq!(jobs.len(), 1, "Should create one job");

    // Verify job has correct label
    let job = &jobs[0];
    let labels = job.metadata.labels.as_ref().expect("Job should have labels");
    assert_eq!(labels.get("cronjob-name"), Some(&"hourly-backup".to_string()));

    // Cleanup
    let _ = storage.delete(&key).await;
    for job in jobs {
        let _ = storage.delete(&build_key("jobs", Some("test-ns"), &job.metadata.name)).await;
    }
}

#[tokio::test]
#[ignore] // Requires running etcd instance
async fn test_cronjob_suspend() {
    let storage = setup_test().await;

    // Clean up
    let _ = storage.delete("/registry/cronjobs/test-ns/suspended").await;
    let jobs: Vec<Job> = storage.list("/registry/jobs/test-ns/").await.unwrap_or_default();
    for job in jobs {
        let _ = storage.delete(&build_key("jobs", Some("test-ns"), &job.metadata.name)).await;
    }

    // Create suspended CronJob
    let mut cronjob = create_test_cronjob("suspended", "test-ns", "@hourly");
    cronjob.spec.suspend = Some(true);
    let key = build_key("cronjobs", Some("test-ns"), "suspended");
    storage.create(&key, &cronjob).await.unwrap();

    // Run controller
    let controller = CronJobController::new(storage.clone());
    controller.reconcile_all().await.unwrap();
    sleep(Duration::from_millis(500)).await;

    // Verify no jobs were created
    let jobs: Vec<Job> = storage.list("/registry/jobs/test-ns/").await.unwrap_or_default();
    assert_eq!(jobs.len(), 0, "Should not create jobs when suspended");

    // Cleanup
    let _ = storage.delete(&key).await;
}

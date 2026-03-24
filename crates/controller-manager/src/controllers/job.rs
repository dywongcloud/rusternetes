use anyhow::Result;
use rusternetes_common::resources::workloads::{Job, JobCondition, JobStatus};
use rusternetes_common::resources::{Pod, PodStatus};
use rusternetes_common::types::{OwnerReference, Phase};
use rusternetes_storage::Storage;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

pub struct JobController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> JobController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting JobController");

        loop {
            if let Err(e) = self.reconcile_all().await {
                error!("Error in Job reconciliation loop: {}", e);
            }
            time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        let jobs: Vec<Job> = self.storage.list("/registry/jobs/").await?;

        for mut job in jobs {
            if let Err(e) = self.reconcile(&mut job).await {
                error!("Failed to reconcile Job {}: {}", job.metadata.name, e);
            }
        }

        Ok(())
    }

    async fn reconcile(&self, job: &mut Job) -> Result<()> {
        let name = &job.metadata.name;
        let namespace = job.metadata.namespace.as_ref().unwrap();

        // Skip reconciliation for Jobs being deleted — GC handles pod cleanup
        if job.metadata.is_being_deleted() {
            return Ok(());
        }

        info!("Reconciling Job {}/{}", namespace, name);

        let completions = job.spec.completions.unwrap_or(1);
        let parallelism = job.spec.parallelism.unwrap_or(1);
        let backoff_limit = job.spec.backoff_limit.unwrap_or(6);

        // Get current pods for this Job
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        // Find pods owned by this Job via ownerReferences (authoritative)
        // Fall back to label matching for backwards compatibility
        let job_uid = &job.metadata.uid;
        let job_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|pod| {
                let owned_by_ref = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| &r.uid == job_uid))
                    .unwrap_or(false);
                let owned_by_label = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("job-name"))
                    .map(|j| j == name)
                    .unwrap_or(false);
                owned_by_ref || owned_by_label
            })
            .collect();

        let mut active = 0;
        let mut succeeded = 0;
        let mut failed = 0;

        for pod in job_pods.iter() {
            if let Some(status) = &pod.status {
                match &status.phase {
                    Some(Phase::Running) | Some(Phase::Pending) => active += 1,
                    Some(Phase::Succeeded) => succeeded += 1,
                    Some(Phase::Failed) => failed += 1,
                    _ => {}
                }
            }
        }

        // For Indexed completion mode, track which indexes have completed
        let is_indexed = job.spec.completion_mode.as_deref() == Some("Indexed");
        let completed_indexes: Option<String> = if is_indexed {
            let mut indexes: Vec<i32> = Vec::new();
            for pod in job_pods.iter() {
                if let Some(status) = &pod.status {
                    if matches!(&status.phase, Some(Phase::Succeeded)) {
                        // Get index from JOB_COMPLETION_INDEX env var or batch.kubernetes.io/job-completion-index annotation
                        let index = pod
                            .metadata
                            .annotations
                            .as_ref()
                            .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                            .and_then(|v| v.parse::<i32>().ok())
                            .or_else(|| {
                                pod.metadata
                                    .labels
                                    .as_ref()
                                    .and_then(|l| l.get("batch.kubernetes.io/job-completion-index"))
                                    .and_then(|v| v.parse::<i32>().ok())
                            })
                            .or_else(|| {
                                // Check env vars on the pod spec
                                pod.spec
                                    .as_ref()
                                    .and_then(|s| {
                                        s.containers.first().and_then(|c| {
                                            c.env.as_ref().and_then(|envs| {
                                                envs.iter()
                                                    .find(|e| e.name == "JOB_COMPLETION_INDEX")
                                                    .and_then(|e| e.value.as_ref())
                                                    .and_then(|v| v.parse::<i32>().ok())
                                            })
                                        })
                                    })
                            });
                        if let Some(idx) = index {
                            indexes.push(idx);
                        }
                    }
                }
            }
            indexes.sort();
            indexes.dedup();
            if indexes.is_empty() {
                None
            } else {
                Some(indexes.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","))
            }
        } else {
            None
        };

        info!(
            "Job {}/{}: active={}, succeeded={}, failed={}, target={}",
            namespace, name, active, succeeded, failed, completions
        );

        // Check if Job is complete
        let is_complete = succeeded >= completions;
        let is_failed = failed > backoff_limit;

        // Preserve the existing start_time if the job was already started
        let existing_start_time = job
            .status
            .as_ref()
            .and_then(|s| s.start_time);

        // Set start_time when the job first has any pods (active, succeeded, or failed)
        let start_time = if active > 0 || succeeded > 0 || failed > 0 {
            Some(existing_start_time.unwrap_or_else(chrono::Utc::now))
        } else {
            existing_start_time
        };

        if is_complete {
            info!("Job {}/{} completed successfully", namespace, name);
            job.status = Some(JobStatus {
                active: Some(0),
                succeeded: Some(succeeded),
                failed: Some(failed),
                conditions: Some(vec![JobCondition {
                    condition_type: "Complete".to_string(),
                    status: "True".to_string(),
                    last_probe_time: Some(chrono::Utc::now()),
                    last_transition_time: Some(chrono::Utc::now()),
                    reason: Some("Completed".to_string()),
                    message: Some("Job completed successfully".to_string()),
                }]),
                start_time,
                completion_time: Some(chrono::Utc::now()),
                ready: None,
                terminating: None,
                completed_indexes: completed_indexes.clone(),
                failed_indexes: None,
                uncounted_terminated_pods: None,
                observed_generation: job.metadata.generation,
            });
        } else if is_failed {
            warn!(
                "Job {}/{} failed after {} failures",
                namespace, name, failed
            );
            job.status = Some(JobStatus {
                active: Some(0),
                succeeded: Some(succeeded),
                failed: Some(failed),
                conditions: Some(vec![JobCondition {
                    condition_type: "Failed".to_string(),
                    status: "True".to_string(),
                    last_probe_time: Some(chrono::Utc::now()),
                    last_transition_time: Some(chrono::Utc::now()),
                    reason: Some("BackoffLimitExceeded".to_string()),
                    message: Some(format!(
                        "Job has reached backoff limit of {}",
                        backoff_limit
                    )),
                }]),
                start_time,
                completion_time: Some(chrono::Utc::now()),
                ready: None,
                terminating: None,
                completed_indexes: completed_indexes.clone(),
                failed_indexes: None,
                uncounted_terminated_pods: None,
                observed_generation: job.metadata.generation,
            });
        } else {
            // Calculate how many new pods to create
            let pods_needed = std::cmp::min(parallelism - active, completions - succeeded - active);

            if pods_needed > 0 {
                // For Indexed mode, find which indexes still need pods
                let mut indexes_to_create: Vec<i32> = if is_indexed {
                    let mut used_indexes: std::collections::HashSet<i32> = std::collections::HashSet::new();
                    for pod in job_pods.iter() {
                        if let Some(idx) = pod.metadata.annotations.as_ref()
                            .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                            .and_then(|v| v.parse::<i32>().ok())
                        {
                            used_indexes.insert(idx);
                        }
                    }
                    (0..completions)
                        .filter(|i| !used_indexes.contains(i))
                        .take(pods_needed as usize)
                        .collect()
                } else {
                    (0..pods_needed).collect()
                };

                for (i, idx) in indexes_to_create.iter().enumerate() {
                    self.create_pod(job, namespace, *idx, is_indexed)
                        .await?;
                    info!(
                        "Created pod for Job {}/{} ({}/{})",
                        namespace,
                        name,
                        job_pods.len() + i as usize + 1,
                        completions
                    );
                }

                // Re-count pods after creation to get accurate status
                let all_pods_after: Vec<Pod> = self.storage.list(&pod_prefix).await?;
                let job_pods_after: Vec<Pod> = all_pods_after
                    .into_iter()
                    .filter(|pod| {
                        pod.metadata
                            .labels
                            .as_ref()
                            .and_then(|labels| labels.get("job-name"))
                            .map(|j| j == name)
                            .unwrap_or(false)
                    })
                    .collect();

                // Recalculate counts
                active = 0;
                succeeded = 0;
                failed = 0;

                for pod in job_pods_after.iter() {
                    if let Some(status) = &pod.status {
                        match &status.phase {
                            Some(Phase::Running) | Some(Phase::Pending) => active += 1,
                            Some(Phase::Succeeded) => succeeded += 1,
                            Some(Phase::Failed) => failed += 1,
                            _ => {}
                        }
                    }
                }
            }

            // Update status
            job.status = Some(JobStatus {
                active: Some(active),
                succeeded: Some(succeeded),
                failed: Some(failed),
                conditions: None,
                start_time,
                completion_time: None,
                ready: None,
                terminating: None,
                completed_indexes: completed_indexes.clone(),
                failed_indexes: None,
                uncounted_terminated_pods: None,
                observed_generation: job.metadata.generation,
            });
        }

        // Save updated status
        let key = format!("/registry/jobs/{}/{}", namespace, name);
        self.storage.update(&key, job).await?;

        Ok(())
    }

    async fn create_pod(&self, job: &Job, namespace: &str, index: i32, is_indexed: bool) -> Result<()> {
        let job_name = &job.metadata.name;
        let pod_name = format!(
            "{}-{}",
            job_name,
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        // Create pod from template
        let template = &job.spec.template;
        let mut labels = template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("job-name".to_string(), job_name.clone());
        labels.insert("controller-uid".to_string(), job.metadata.uid.clone());
        if is_indexed {
            labels.insert("batch.kubernetes.io/job-completion-index".to_string(), index.to_string());
        }

        let mut annotations = template
            .metadata
            .as_ref()
            .and_then(|m| m.annotations.clone())
            .unwrap_or_default();
        if is_indexed {
            annotations.insert("batch.kubernetes.io/job-completion-index".to_string(), index.to_string());
        }

        let mut spec = template.spec.clone();
        // Jobs should not restart on failure - let the Job controller handle retries
        spec.restart_policy = Some("Never".to_string());

        // For Indexed mode, inject JOB_COMPLETION_INDEX env var into all containers
        if is_indexed {
            for container in &mut spec.containers {
                let env = container.env.get_or_insert_with(Vec::new);
                if !env.iter().any(|e| e.name == "JOB_COMPLETION_INDEX") {
                    env.push(rusternetes_common::resources::EnvVar {
                        name: "JOB_COMPLETION_INDEX".to_string(),
                        value: Some(index.to_string()),
                        value_from: None,
                    });
                }
            }
        }

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: pod_name.clone(),
                generate_name: None,
                generation: None,
                managed_fields: None,
                namespace: Some(namespace.to_string()),
                labels: Some(labels),
                annotations: Some(annotations),
                uid: uuid::Uuid::new_v4().to_string(),
                creation_timestamp: Some(chrono::Utc::now()),
                deletion_timestamp: None,
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: Some(vec![OwnerReference {
                    api_version: "batch/v1".to_string(),
                    kind: "Job".to_string(),
                    name: job_name.clone(),
                    uid: job.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]),
            },
            spec: Some(spec),
            status: Some(PodStatus {
                phase: Some(Phase::Pending),
                message: None,
                reason: None,
                pod_ip: None,
                host_ip: None,
                host_i_ps: None,
                pod_i_ps: None,
                nominated_node_name: None,
                qos_class: None,
                start_time: None,
                conditions: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
                resize: None,
                resource_claim_statuses: None,
                observed_generation: None,
            }),
        };

        let key = format!("/registry/pods/{}/{}", namespace, pod_name);
        self.storage.create(&key, &pod).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pods_needed_calculation() {
        let parallelism = 3;
        let completions = 10;
        let active = 2;
        let succeeded = 5;

        let pods_needed = std::cmp::min(
            parallelism - active,             // 1 (can run 1 more in parallel)
            completions - succeeded - active, // 3 (need 3 more to complete)
        );

        assert_eq!(pods_needed, 1);
    }

    #[test]
    fn test_job_completion() {
        let completions = 5;
        let succeeded = 5;
        assert!(succeeded >= completions);
    }

    #[test]
    fn test_backoff_limit_exceeded() {
        let backoff_limit = 6;
        let failed = 7;
        assert!(failed > backoff_limit);
    }
}

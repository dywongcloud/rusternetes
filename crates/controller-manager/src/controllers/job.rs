use anyhow::Result;
use rusternetes_common::resources::workloads::{Job, JobCondition, JobStatus};
use rusternetes_common::resources::{Pod, PodStatus};
use rusternetes_common::types::{OwnerReference, Phase};
use rusternetes_storage::{build_key, Storage};
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
            time::sleep(Duration::from_secs(2)).await;
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

        // Release pods whose labels no longer match the Job selector
        let selector = job.spec.selector.as_ref().and_then(|s| s.match_labels.as_ref());
        for pod in &job_pods {
            if let Some(sel) = selector {
                let labels = pod.metadata.labels.as_ref();
                let matches = labels.map_or(false, |l| sel.iter().all(|(k, v)| l.get(k) == Some(v)));
                if !matches {
                    // Pod no longer matches selector — release it by removing ownerReference
                    let mut released = pod.clone();
                    if let Some(ref mut refs) = released.metadata.owner_references {
                        refs.retain(|r| &r.uid != job_uid);
                    }
                    let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                    if let Err(e) = self.storage.update(&pod_key, &released).await {
                        tracing::warn!("Failed to release pod {}: {}", pod.metadata.name, e);
                    } else {
                        info!("Released pod {} from job {}/{} (labels no longer match)", pod.metadata.name, namespace, name);
                    }
                    continue;
                }
            }
        }

        // Adopt orphaned pods — re-add ownerReference if pod matches by label but not by ownerRef
        for pod in &job_pods {
            let has_owner_ref = pod.metadata.owner_references.as_ref()
                .map(|refs| refs.iter().any(|r| &r.uid == job_uid))
                .unwrap_or(false);
            if !has_owner_ref {
                let mut adopted_pod = pod.clone();
                let owner_ref = rusternetes_common::types::OwnerReference {
                    api_version: "batch/v1".to_string(),
                    kind: "Job".to_string(),
                    name: name.to_string(),
                    uid: job.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                };
                adopted_pod.metadata.owner_references
                    .get_or_insert_with(Vec::new)
                    .push(owner_ref);
                let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                if let Err(e) = self.storage.update(&pod_key, &adopted_pod).await {
                    tracing::warn!("Failed to adopt pod {}: {}", pod.metadata.name, e);
                } else {
                    info!("Adopted orphaned pod {} for job {}/{}", pod.metadata.name, namespace, name);
                }
            }
        }

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

        // Track failed indexes for backoffLimitPerIndex
        let backoff_limit_per_index = job.spec.backoff_limit_per_index;
        let failed_indexes: Option<String> = if is_indexed && backoff_limit_per_index.is_some() {
            let per_index_limit = backoff_limit_per_index.unwrap_or(0);
            let mut failed_idx: Vec<i32> = Vec::new();
            // Count failures per index
            let mut failures_per_index: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
            for pod in job_pods.iter() {
                if let Some(status) = &pod.status {
                    if matches!(&status.phase, Some(Phase::Failed)) {
                        let index = pod.metadata.annotations.as_ref()
                            .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                            .and_then(|v| v.parse::<i32>().ok())
                            .unwrap_or(-1);
                        if index >= 0 {
                            *failures_per_index.entry(index).or_insert(0) += 1;
                        }
                    }
                }
            }
            for (idx, count) in &failures_per_index {
                if *count > per_index_limit {
                    failed_idx.push(*idx);
                }
            }
            failed_idx.sort();
            if failed_idx.is_empty() { None } else {
                Some(failed_idx.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","))
            }
        } else {
            None
        };

        info!(
            "Job {}/{}: active={}, succeeded={}, failed={}, target={}",
            namespace, name, active, succeeded, failed, completions
        );

        // Check podFailurePolicy — if a failed pod matches a FailJob rule, fail immediately
        let mut pod_failure_policy_triggered = false;
        let mut pod_failure_message = String::new();
        if let Some(ref policy) = job.spec.pod_failure_policy {
            if let Some(rules) = policy.get("rules").and_then(|r| r.as_array()) {
                for pod in job_pods.iter() {
                    let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                    if !matches!(phase, Some(Phase::Failed)) {
                        continue;
                    }
                    // Get container exit codes
                    let exit_codes: Vec<i32> = pod.status.as_ref()
                        .and_then(|s| s.container_statuses.as_ref())
                        .map(|cs| cs.iter().filter_map(|c| {
                            match &c.state {
                                Some(rusternetes_common::resources::ContainerState::Terminated { exit_code, .. }) => Some(*exit_code),
                                _ => None,
                            }
                        }).collect())
                        .unwrap_or_default();

                    for rule in rules {
                        let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("");
                        if action != "FailJob" && action != "FailIndex" { continue; }

                        // Check onExitCodes
                        if let Some(on_exit) = rule.get("onExitCodes") {
                            let operator = on_exit.get("operator").and_then(|o| o.as_str()).unwrap_or("In");
                            let values: Vec<i32> = on_exit.get("values")
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect())
                                .unwrap_or_default();
                            let matches = exit_codes.iter().any(|code| {
                                match operator {
                                    "In" => values.contains(code),
                                    "NotIn" => !values.contains(code),
                                    _ => false,
                                }
                            });
                            if matches {
                                if action == "FailJob" {
                                    pod_failure_policy_triggered = true;
                                    pod_failure_message = format!("Pod failed with exit code matching FailJob rule");
                                } else if action == "FailIndex" && is_indexed {
                                    // Mark this pod's index as failed
                                    let index = pod.metadata.annotations.as_ref()
                                        .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                                        .and_then(|v| v.parse::<i32>().ok());
                                    if let Some(idx) = index {
                                        // Add to failed count — will be checked in is_failed logic
                                        failed += 1;
                                    }
                                }
                                break;
                            }
                        }

                        // Check onPodConditions
                        if let Some(on_conditions) = rule.get("onPodConditions") {
                            if let Some(conditions) = on_conditions.as_array() {
                                let pod_conditions = pod.status.as_ref()
                                    .and_then(|s| s.conditions.as_ref());
                                let matches = conditions.iter().any(|cond| {
                                    let ctype = cond.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                    let cstatus = cond.get("status").and_then(|s| s.as_str()).unwrap_or("True");
                                    pod_conditions.map(|pcs| pcs.iter().any(|pc| {
                                        pc.condition_type == ctype && pc.status == cstatus
                                    })).unwrap_or(false)
                                });
                                if matches {
                                    if action == "FailJob" {
                                        pod_failure_policy_triggered = true;
                                        pod_failure_message = format!("Pod condition matched FailJob rule");
                                    }
                                    // FailIndex is handled by the is_failed logic below
                                    break;
                                }
                            }
                        }
                    }
                    if pod_failure_policy_triggered { break; }
                }
            }
        }

        // Check if Job is complete
        let is_complete = succeeded >= completions;

        // Check maxFailedIndexes — if the number of failed indexes exceeds this limit, fail the job
        let max_failed_indexes_exceeded = if is_indexed {
            if let Some(max_failed) = job.spec.max_failed_indexes {
                let failed_index_count = failed_indexes.as_ref().map(|s| s.split(',').count()).unwrap_or(0) as i32;
                // Also count indexes that failed but aren't tracked in failed_indexes (no backoffLimitPerIndex)
                if backoff_limit_per_index.is_none() {
                    // Without backoffLimitPerIndex, count unique failed indexes
                    let mut failed_idx_set: std::collections::HashSet<i32> = std::collections::HashSet::new();
                    for pod in job_pods.iter() {
                        if matches!(pod.status.as_ref().and_then(|s| s.phase.as_ref()), Some(Phase::Failed)) {
                            let index = pod.metadata.annotations.as_ref()
                                .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                                .and_then(|v| v.parse::<i32>().ok())
                                .unwrap_or(-1);
                            if index >= 0 { failed_idx_set.insert(index); }
                        }
                    }
                    failed_idx_set.len() as i32 > max_failed
                } else {
                    failed_index_count > max_failed
                }
            } else {
                false
            }
        } else {
            false
        };

        // For backoffLimitPerIndex, job fails when all indexes are either succeeded or failed
        let is_failed = pod_failure_policy_triggered || max_failed_indexes_exceeded || if backoff_limit_per_index.is_some() && is_indexed {
            let completed_count = completed_indexes.as_ref().map(|s| s.split(',').count()).unwrap_or(0);
            let failed_count = failed_indexes.as_ref().map(|s| s.split(',').count()).unwrap_or(0);
            (completed_count + failed_count) as i32 >= completions
        } else {
            failed > backoff_limit
        };

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

        // Check successPolicy — if defined and criteria met, mark job complete
        let success_policy_met = if let Some(ref policy) = job.spec.success_policy {
            if let Some(rules) = policy.get("rules").and_then(|r| r.as_array()) {
                rules.iter().any(|rule| {
                    // Check succeededIndexes (specific indexes that must succeed)
                    if let Some(succeeded_indexes) = rule.get("succeededIndexes").and_then(|s| s.as_str()) {
                        let completed = completed_indexes.as_deref().unwrap_or("");
                        let completed_set: std::collections::HashSet<i32> = completed.split(',')
                            .filter_map(|s| s.trim().parse::<i32>().ok())
                            .collect();
                        // Parse required indexes (supports ranges like "0-3" and individual "0,1")
                        let mut required_met = true;
                        for part in succeeded_indexes.split(',') {
                            let part = part.trim();
                            if part.contains('-') {
                                let bounds: Vec<&str> = part.split('-').collect();
                                if bounds.len() == 2 {
                                    if let (Ok(start), Ok(end)) = (bounds[0].parse::<i32>(), bounds[1].parse::<i32>()) {
                                        for i in start..=end {
                                            if !completed_set.contains(&i) {
                                                required_met = false;
                                                break;
                                            }
                                        }
                                    }
                                }
                            } else if let Ok(idx) = part.parse::<i32>() {
                                if !completed_set.contains(&idx) {
                                    required_met = false;
                                }
                            }
                            if !required_met { break; }
                        }
                        required_met
                    }
                    // Check succeededCount (minimum number of succeeded pods)
                    else if let Some(count) = rule.get("succeededCount").and_then(|c| c.as_i64()) {
                        succeeded >= count as i32
                    }
                    // No criteria specified — rule matches when all completions succeed
                    else {
                        succeeded >= completions
                    }
                })
            } else {
                false
            }
        } else {
            false
        };

        if success_policy_met {
            info!("Job {}/{} met success policy criteria", namespace, name);

            // When successPolicy triggers, terminate remaining active pods
            // and only count pods that match the success criteria
            let success_indexes: std::collections::HashSet<i32> = if let Some(ref policy) = job.spec.success_policy {
                policy.get("rules").and_then(|r| r.as_array())
                    .and_then(|rules| rules.first())
                    .and_then(|rule| rule.get("succeededIndexes").and_then(|s| s.as_str()))
                    .map(|indexes| {
                        let mut set = std::collections::HashSet::new();
                        for part in indexes.split(',') {
                            let part = part.trim();
                            if part.contains('-') {
                                let bounds: Vec<&str> = part.split('-').collect();
                                if bounds.len() == 2 {
                                    if let (Ok(start), Ok(end)) = (bounds[0].parse::<i32>(), bounds[1].parse::<i32>()) {
                                        for i in start..=end { set.insert(i); }
                                    }
                                }
                            } else if let Ok(idx) = part.parse::<i32>() {
                                set.insert(idx);
                            }
                        }
                        set
                    })
                    .unwrap_or_default()
            } else {
                std::collections::HashSet::new()
            };

            // Count only pods matching the success criteria indexes
            let policy_succeeded = if !success_indexes.is_empty() && is_indexed {
                job_pods.iter().filter(|pod| {
                    let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                    if !matches!(phase, Some(Phase::Succeeded)) { return false; }
                    let index = pod.metadata.annotations.as_ref()
                        .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                        .and_then(|v| v.parse::<i32>().ok())
                        .unwrap_or(-1);
                    success_indexes.contains(&index)
                }).count() as i32
            } else {
                succeeded
            };

            // Terminate remaining active pods
            for pod in job_pods.iter() {
                let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                if matches!(phase, Some(Phase::Running) | Some(Phase::Pending)) {
                    let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                    let mut term_pod = pod.clone();
                    term_pod.metadata.deletion_timestamp = Some(chrono::Utc::now());
                    let _ = self.storage.update(&pod_key, &term_pod).await;
                }
            }

            job.status = Some(JobStatus {
                active: Some(0),
                succeeded: Some(policy_succeeded),
                failed: Some(failed),
                conditions: Some(vec![
                    JobCondition {
                        condition_type: "SuccessCriteriaMet".to_string(),
                        status: "True".to_string(),
                        last_probe_time: Some(chrono::Utc::now()),
                        last_transition_time: Some(chrono::Utc::now()),
                        reason: Some("SuccessPolicy".to_string()),
                        message: Some("Job met success policy criteria".to_string()),
                    },
                    JobCondition {
                        condition_type: "Complete".to_string(),
                        status: "True".to_string(),
                        last_probe_time: Some(chrono::Utc::now()),
                        last_transition_time: Some(chrono::Utc::now()),
                        reason: Some("SuccessPolicy".to_string()),
                        message: Some("Job completed via success policy".to_string()),
                    },
                ]),
                start_time,
                completion_time: Some(chrono::Utc::now()),
                ready: None,
                terminating: Some(active),
                completed_indexes: completed_indexes.clone(),
                failed_indexes: failed_indexes.clone(),
                uncounted_terminated_pods: None,
                observed_generation: job.metadata.generation,
            });
            let key = format!("/registry/jobs/{}/{}", namespace, name);
            self.storage.update(&key, job).await?;
            return Ok(());
        }

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
                failed_indexes: failed_indexes.clone(),
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
                    reason: Some(if pod_failure_policy_triggered { "PodFailurePolicy".to_string() } else { "BackoffLimitExceeded".to_string() }),
                    message: Some(if pod_failure_policy_triggered {
                        pod_failure_message.clone()
                    } else {
                        format!("Job has reached backoff limit of {}", backoff_limit)
                    }),
                }]),
                start_time,
                completion_time: Some(chrono::Utc::now()),
                ready: None,
                terminating: None,
                completed_indexes: completed_indexes.clone(),
                failed_indexes: failed_indexes.clone(),
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

            // Update status — but preserve conditions and completion_time if job
            // was already completed (e.g. by SuccessPolicy). Otherwise the regular
            // update path overwrites the completion status.
            let existing_conditions = job.status.as_ref().and_then(|s| s.conditions.clone());
            let existing_completion = job.status.as_ref().and_then(|s| s.completion_time);
            let already_complete = existing_conditions.as_ref()
                .map(|c| c.iter().any(|cond| cond.condition_type == "Complete" && cond.status == "True"))
                .unwrap_or(false);

            if !already_complete {
                job.status = Some(JobStatus {
                    active: Some(active),
                    succeeded: Some(succeeded),
                    failed: Some(failed),
                    conditions: existing_conditions,
                    start_time,
                    completion_time: existing_completion,
                    ready: None,
                    terminating: None,
                    completed_indexes: completed_indexes.clone(),
                    failed_indexes: failed_indexes.clone(),
                    uncounted_terminated_pods: None,
                    observed_generation: job.metadata.generation,
                });
            }
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

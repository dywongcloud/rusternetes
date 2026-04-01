use anyhow::Result;
use rusternetes_common::resources::workloads::{Job, JobCondition, JobStatus};
use rusternetes_common::resources::{Pod, PodStatus};
use rusternetes_common::types::{OwnerReference, Phase};
use rusternetes_storage::{build_key, Storage};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

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

        // Skip already-completed or already-failed jobs
        if let Some(ref status) = job.status {
            if let Some(ref conditions) = status.conditions {
                let is_finished = conditions.iter().any(|c| {
                    (c.condition_type == "Complete" || c.condition_type == "Failed")
                        && c.status == "True"
                });
                if is_finished {
                    return Ok(());
                }
            }
        }

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
        let selector = job
            .spec
            .selector
            .as_ref()
            .and_then(|s| s.match_labels.as_ref());
        if selector.is_none() && !job_pods.is_empty() {
            debug!(
                "Job {}/{} has no matchLabels selector, skipping release check for {} pods",
                namespace, name, job_pods.len()
            );
        }
        for pod in &job_pods {
            if let Some(sel) = selector {
                let labels = pod.metadata.labels.as_ref();
                let matches =
                    labels.map_or(false, |l| sel.iter().all(|(k, v)| l.get(k) == Some(v)));
                if !matches {
                    // Pod no longer matches selector — release it by removing ownerReference
                    // Use CAS retry to handle concurrent updates
                    let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                    for _ in 0..3 {
                        match self.storage.get::<Pod>(&pod_key).await {
                            Ok(mut fresh_pod) => {
                                if let Some(ref mut refs) = fresh_pod.metadata.owner_references {
                                    refs.retain(|r| &r.uid != job_uid);
                                }
                                match self.storage.update(&pod_key, &fresh_pod).await {
                                    Ok(_) => {
                                        info!(
                                            "Released pod {} from job {}/{} (labels no longer match)",
                                            pod.metadata.name, namespace, name
                                        );
                                        break;
                                    }
                                    Err(e) => {
                                        warn!("CAS retry releasing pod {}: {}", pod.metadata.name, e);
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    continue;
                }
            }
        }

        // Adopt orphaned pods — re-add ownerReference if pod matches by label but not by ownerRef
        for pod in &job_pods {
            let has_owner_ref = pod
                .metadata
                .owner_references
                .as_ref()
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
                adopted_pod
                    .metadata
                    .owner_references
                    .get_or_insert_with(Vec::new)
                    .push(owner_ref);
                let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                if let Err(e) = self.storage.update(&pod_key, &adopted_pod).await {
                    tracing::warn!("Failed to adopt pod {}: {}", pod.metadata.name, e);
                } else {
                    info!(
                        "Adopted orphaned pod {} for job {}/{}",
                        pod.metadata.name, namespace, name
                    );
                }
            }
        }

        let is_indexed = job.spec.completion_mode.as_deref() == Some("Indexed");

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

        // Handle suspended jobs: delete all active pods and set active to 0
        if job.spec.suspend.unwrap_or(false) {
            if active > 0 {
                for pod in job_pods.iter() {
                    let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                    if matches!(phase, Some(Phase::Running) | Some(Phase::Pending)) {
                        let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                        let _ = self.storage.delete(&pod_key).await;
                        info!(
                            "Suspended job {}/{}: deleted active pod {}",
                            namespace, name, pod.metadata.name
                        );
                    }
                }
            }
            // Preserve existing start_time
            let existing_start_time = job.status.as_ref().and_then(|s| s.start_time);
            let existing_conditions = job.status.as_ref().and_then(|s| s.conditions.clone());
            job.status = Some(JobStatus {
                active: Some(0),
                succeeded: Some(succeeded),
                failed: Some(failed),
                conditions: existing_conditions,
                start_time: existing_start_time,
                completion_time: None,
                ready: None,
                terminating: None,
                completed_indexes: None,
                failed_indexes: None,
                uncounted_terminated_pods: None,
                observed_generation: job.metadata.generation,
            });
            let key = format!("/registry/jobs/{}/{}", namespace, name);
            // Refresh RV before update to avoid CAS conflict
            if let Ok(mut fresh) = self.storage.get::<Job>(&key).await {
                fresh.status = job.status.clone();
                self.storage.update(&key, &fresh).await?;
            }
            return Ok(());
        }

        // Handle activeDeadlineSeconds — fail the job if it has been active too long
        if let Some(deadline) = job.spec.active_deadline_seconds {
            if let Some(start) = job.status.as_ref().and_then(|s| s.start_time) {
                let elapsed = chrono::Utc::now()
                    .signed_duration_since(start)
                    .num_seconds();
                if elapsed > deadline {
                    warn!(
                        "Job {}/{} exceeded activeDeadlineSeconds ({} > {})",
                        namespace, name, elapsed, deadline
                    );
                    // Delete all active pods
                    for pod in job_pods.iter() {
                        let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                        if matches!(phase, Some(Phase::Running) | Some(Phase::Pending)) {
                            let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                            let _ = self.storage.delete(&pod_key).await;
                        }
                    }
                    job.status = Some(JobStatus {
                        active: Some(0),
                        succeeded: Some(succeeded),
                        failed: Some(failed),
                        conditions: Some(vec![JobCondition {
                            condition_type: "Failed".to_string(),
                            status: "True".to_string(),
                            last_probe_time: Some(chrono::Utc::now()),
                            last_transition_time: Some(chrono::Utc::now()),
                            reason: Some("DeadlineExceeded".to_string()),
                            message: Some(format!(
                                "Job was active longer than specified deadline of {} seconds",
                                deadline
                            )),
                        }]),
                        start_time: job.status.as_ref().and_then(|s| s.start_time),
                        completion_time: Some(chrono::Utc::now()),
                        ready: None,
                        terminating: None,
                        completed_indexes: None,
                        failed_indexes: None,
                        uncounted_terminated_pods: None,
                        observed_generation: job.metadata.generation,
                    });
                    let key = format!("/registry/jobs/{}/{}", namespace, name);
                    // Refresh RV before update to avoid CAS conflict
                    if let Ok(mut fresh) = self.storage.get::<Job>(&key).await {
                        fresh.status = job.status.clone();
                        self.storage.update(&key, &fresh).await?;
                    }
                    return Ok(());
                }
            }
        }

        // Helper: extract completion index from a pod
        fn get_pod_index(pod: &Pod) -> Option<i32> {
            pod.metadata
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
                    pod.spec.as_ref().and_then(|s| {
                        s.containers.first().and_then(|c| {
                            c.env.as_ref().and_then(|envs| {
                                envs.iter()
                                    .find(|e| e.name == "JOB_COMPLETION_INDEX")
                                    .and_then(|e| e.value.as_ref())
                                    .and_then(|v| v.parse::<i32>().ok())
                            })
                        })
                    })
                })
        }

        // For Indexed completion mode, track which indexes have completed
        let completed_indexes: Option<String> = if is_indexed {
            let mut indexes: Vec<i32> = Vec::new();
            for pod in job_pods.iter() {
                if let Some(status) = &pod.status {
                    if matches!(&status.phase, Some(Phase::Succeeded)) {
                        if let Some(idx) = get_pod_index(pod) {
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
                Some(format_index_ranges(&indexes))
            }
        } else {
            None
        };

        // Build a set of indexes that failed due to FailIndex podFailurePolicy
        let mut fail_index_set: HashSet<i32> = HashSet::new();

        // Check podFailurePolicy — if a failed pod matches a FailJob rule, fail immediately
        // Also check for FailIndex rules
        let mut pod_failure_policy_triggered = false;
        let mut pod_failure_message = String::new();
        if let Some(ref policy) = job.spec.pod_failure_policy {
            if let Some(rules) = policy.get("rules").and_then(|r| r.as_array()) {
                for pod in job_pods.iter() {
                    let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                    // Check ALL terminated pods (Failed AND Succeeded with non-zero exit).
                    // K8s podFailurePolicy evaluates against any pod with terminated containers,
                    // not just Failed phase pods. A pod can be Succeeded but have containers
                    // that exited with non-zero codes (if other containers succeeded).
                    if !matches!(phase, Some(Phase::Failed) | Some(Phase::Succeeded)) {
                        continue;
                    }
                    // Get container exit codes
                    let exit_codes: Vec<i32> = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.container_statuses.as_ref())
                        .map(|cs| {
                            cs.iter()
                                .filter_map(|c| match &c.state {
                                    Some(
                                        rusternetes_common::resources::ContainerState::Terminated {
                                            exit_code,
                                            ..
                                        },
                                    ) => Some(*exit_code),
                                    _ => None,
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    for rule in rules {
                        let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("");

                        let mut rule_matched = false;

                        // Check onExitCodes
                        if let Some(on_exit) = rule.get("onExitCodes") {
                            let operator = on_exit
                                .get("operator")
                                .and_then(|o| o.as_str())
                                .unwrap_or("In");
                            let values: Vec<i32> = on_exit
                                .get("values")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_i64().map(|i| i as i32))
                                        .collect()
                                })
                                .unwrap_or_default();
                            rule_matched = exit_codes.iter().any(|code| match operator {
                                "In" => values.contains(code),
                                "NotIn" => !values.contains(code),
                                _ => false,
                            });
                        }

                        // Check onPodConditions
                        if !rule_matched {
                            if let Some(on_conditions) = rule.get("onPodConditions") {
                                if let Some(conditions) = on_conditions.as_array() {
                                    let pod_conditions =
                                        pod.status.as_ref().and_then(|s| s.conditions.as_ref());
                                    rule_matched = conditions.iter().any(|cond| {
                                        let ctype =
                                            cond.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                        let cstatus = cond
                                            .get("status")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("True");
                                        pod_conditions
                                            .map(|pcs| {
                                                pcs.iter().any(|pc| {
                                                    pc.condition_type == ctype
                                                        && pc.status == cstatus
                                                })
                                            })
                                            .unwrap_or(false)
                                    });
                                }
                            }
                        }

                        if rule_matched {
                            match action {
                                "FailJob" => {
                                    pod_failure_policy_triggered = true;
                                    pod_failure_message =
                                        "Pod failed with exit code matching FailJob rule"
                                            .to_string();
                                    break;
                                }
                                "FailIndex" => {
                                    if is_indexed {
                                        if let Some(idx) = get_pod_index(pod) {
                                            fail_index_set.insert(idx);
                                        }
                                    }
                                    break;
                                }
                                _ => {} // Count, Ignore, etc.
                            }
                        }
                    }
                    if pod_failure_policy_triggered {
                        break;
                    }
                }
            }
        }

        // Track failed indexes for backoffLimitPerIndex
        let backoff_limit_per_index = job.spec.backoff_limit_per_index;
        let mut backoff_failed_index_set: HashSet<i32> = HashSet::new();

        if is_indexed && backoff_limit_per_index.is_some() {
            let per_index_limit = backoff_limit_per_index.unwrap_or(0);
            // Count failures per index
            let mut failures_per_index: HashMap<i32, i32> = HashMap::new();
            for pod in job_pods.iter() {
                if let Some(status) = &pod.status {
                    if matches!(&status.phase, Some(Phase::Failed)) {
                        if let Some(index) = get_pod_index(pod) {
                            *failures_per_index.entry(index).or_insert(0) += 1;
                        }
                    }
                }
            }
            for (idx, count) in &failures_per_index {
                if *count > per_index_limit {
                    backoff_failed_index_set.insert(*idx);
                }
            }
        }

        // Merge FailIndex and backoff-per-index failed sets
        let all_failed_index_set: HashSet<i32> = fail_index_set
            .union(&backoff_failed_index_set)
            .copied()
            .collect();

        let failed_indexes: Option<String> = if !all_failed_index_set.is_empty() {
            let mut sorted: Vec<i32> = all_failed_index_set.iter().copied().collect();
            sorted.sort();
            Some(format_index_ranges(&sorted))
        } else {
            None
        };

        // Count succeeded indexes for Indexed mode
        let succeeded_index_count = if is_indexed {
            let mut idx_set: HashSet<i32> = HashSet::new();
            for pod in job_pods.iter() {
                if matches!(
                    pod.status.as_ref().and_then(|s| s.phase.as_ref()),
                    Some(Phase::Succeeded)
                ) {
                    if let Some(idx) = get_pod_index(pod) {
                        idx_set.insert(idx);
                    }
                }
            }
            idx_set.len() as i32
        } else {
            succeeded
        };

        info!(
            "Job {}/{}: active={}, succeeded={}, failed={}, target={}",
            namespace, name, active, succeeded, failed, completions
        );

        // Check if Job is complete
        // For indexed jobs, check number of distinct succeeded indexes
        let is_complete = if is_indexed {
            succeeded_index_count >= completions
        } else {
            succeeded >= completions
        };

        // Check maxFailedIndexes — if the number of failed indexes exceeds this limit, fail the job
        let max_failed_indexes_exceeded = if is_indexed {
            if let Some(max_failed) = job.spec.max_failed_indexes {
                let failed_index_count = all_failed_index_set.len() as i32;
                // Also count unique indexes with only failed pods (no succeeded) when no backoffLimitPerIndex
                if backoff_limit_per_index.is_none() && fail_index_set.is_empty() {
                    let mut failed_idx_set: HashSet<i32> = HashSet::new();
                    for pod in job_pods.iter() {
                        if matches!(
                            pod.status.as_ref().and_then(|s| s.phase.as_ref()),
                            Some(Phase::Failed)
                        ) {
                            if let Some(index) = get_pod_index(pod) {
                                failed_idx_set.insert(index);
                            }
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
        let is_failed = pod_failure_policy_triggered
            || max_failed_indexes_exceeded
            || if backoff_limit_per_index.is_some() && is_indexed {
                let completed_count = succeeded_index_count;
                let failed_count = all_failed_index_set.len() as i32;
                (completed_count + failed_count) >= completions
            } else {
                failed > backoff_limit
            };

        // Preserve the existing start_time if the job was already started
        let existing_start_time = job.status.as_ref().and_then(|s| s.start_time);

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
                    let indexes_ok = if let Some(succeeded_indexes_str) =
                        rule.get("succeededIndexes").and_then(|s| s.as_str())
                    {
                        // Parse required indexes and check they are all in completed set
                        let completed = completed_indexes.as_deref().unwrap_or("");
                        let completed_set: HashSet<i32> = parse_index_ranges(completed);
                        let required_set: HashSet<i32> = parse_index_ranges(succeeded_indexes_str);
                        required_set.is_subset(&completed_set)
                    } else {
                        true // No index constraint
                    };

                    let count_ok =
                        if let Some(count) = rule.get("succeededCount").and_then(|c| c.as_i64()) {
                            succeeded_index_count >= count as i32
                        } else {
                            true // No count constraint
                        };

                    // If rule has neither succeededIndexes nor succeededCount, match on all completions
                    let has_criteria = rule.get("succeededIndexes").is_some()
                        || rule.get("succeededCount").is_some();
                    if has_criteria {
                        indexes_ok && count_ok
                    } else {
                        succeeded_index_count >= completions
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
                succeeded: Some(succeeded),
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
            // Refresh RV before update to avoid CAS conflict
            if let Ok(mut fresh) = self.storage.get::<Job>(&key).await {
                fresh.status = job.status.clone();
                self.storage.update(&key, &fresh).await?;
            }
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

            // Determine failure reason
            let (reason, message) = if pod_failure_policy_triggered {
                ("PodFailurePolicy".to_string(), pod_failure_message.clone())
            } else if max_failed_indexes_exceeded {
                (
                    "MaxFailedIndexesExceeded".to_string(),
                    "Job has exceeded the maximum number of failed indexes".to_string(),
                )
            } else if backoff_limit_per_index.is_some() && is_indexed {
                (
                    "FailedIndexes".to_string(),
                    format!(
                        "Job has failed indexes: {}",
                        failed_indexes.as_deref().unwrap_or("")
                    ),
                )
            } else {
                (
                    "BackoffLimitExceeded".to_string(),
                    format!("Job has reached backoff limit of {}", backoff_limit),
                )
            };

            job.status = Some(JobStatus {
                active: Some(0),
                succeeded: Some(succeeded),
                failed: Some(failed),
                conditions: Some(vec![JobCondition {
                    condition_type: "Failed".to_string(),
                    status: "True".to_string(),
                    last_probe_time: Some(chrono::Utc::now()),
                    last_transition_time: Some(chrono::Utc::now()),
                    reason: Some(reason),
                    message: Some(message),
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
                let indexes_to_create: Vec<i32> = if is_indexed {
                    // Track indexes that already have active or succeeded pods
                    let mut active_or_succeeded_indexes: HashSet<i32> = HashSet::new();
                    for pod in job_pods.iter() {
                        let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                        if matches!(
                            phase,
                            Some(Phase::Running) | Some(Phase::Pending) | Some(Phase::Succeeded)
                        ) {
                            if let Some(idx) = get_pod_index(pod) {
                                active_or_succeeded_indexes.insert(idx);
                            }
                        }
                    }
                    (0..completions)
                        .filter(|i| {
                            // Skip indexes that already have active or succeeded pods
                            if active_or_succeeded_indexes.contains(i) {
                                return false;
                            }
                            // Skip indexes that are permanently failed (backoffLimitPerIndex or FailIndex)
                            if all_failed_index_set.contains(i) {
                                return false;
                            }
                            true
                        })
                        .take(pods_needed as usize)
                        .collect()
                } else {
                    (0..pods_needed).collect()
                };

                for (i, idx) in indexes_to_create.iter().enumerate() {
                    self.create_pod(job, namespace, *idx, is_indexed).await?;
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
            let already_complete = existing_conditions
                .as_ref()
                .map(|c| {
                    c.iter()
                        .any(|cond| cond.condition_type == "Complete" && cond.status == "True")
                })
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
        let has_complete = job
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .map(|c| {
                c.iter()
                    .any(|cond| cond.condition_type == "Complete" && cond.status == "True")
            })
            .unwrap_or(false);
        let has_failed = job
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .map(|c| {
                c.iter()
                    .any(|cond| cond.condition_type == "Failed" && cond.status == "True")
            })
            .unwrap_or(false);
        if has_complete || has_failed {
            info!(
                "Job {}/{} status update: complete={}, failed={}, conditions={:?}",
                namespace,
                name,
                has_complete,
                has_failed,
                job.status.as_ref().and_then(|s| s.conditions.as_ref())
            );
        }
        // Refresh resourceVersion before update to avoid CAS conflicts.
        // The job's RV may be stale from the list() at the start of reconcile_all().
        // Other components (API server, kubelet) may have modified the job since then.
        let status_to_save = job.status.clone();
        for attempt in 0..3 {
            match self.storage.get::<Job>(&key).await {
                Ok(mut fresh_job) => {
                    fresh_job.status = status_to_save.clone();
                    match self.storage.update(&key, &fresh_job).await {
                        Ok(_) => {
                            if has_complete || has_failed {
                                info!(
                                    "Job {}/{} status update persisted (attempt {})",
                                    namespace, name, attempt + 1
                                );
                            }
                            break;
                        }
                        Err(e) => {
                            warn!(
                                "Job {}/{} status update CAS conflict (attempt {}): {}",
                                namespace, name, attempt + 1, e
                            );
                            if attempt == 2 {
                                return Err(e.into());
                            }
                        }
                    }
                }
                Err(e) => {
                    // Job was deleted between list and update
                    debug!("Job {}/{} no longer exists: {}", namespace, name, e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn create_pod(
        &self,
        job: &Job,
        namespace: &str,
        index: i32,
        is_indexed: bool,
    ) -> Result<()> {
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
            labels.insert(
                "batch.kubernetes.io/job-completion-index".to_string(),
                index.to_string(),
            );
        }

        let mut annotations = template
            .metadata
            .as_ref()
            .and_then(|m| m.annotations.clone())
            .unwrap_or_default();
        if is_indexed {
            annotations.insert(
                "batch.kubernetes.io/job-completion-index".to_string(),
                index.to_string(),
            );
        }

        let mut spec = template.spec.clone();

        // Respect the template's restart policy.
        // For restartPolicy: OnFailure, the kubelet will restart failed containers in-place,
        // allowing the pod to eventually succeed without the Job controller creating new pods.
        // For restartPolicy: Never (or if not set), the Job controller handles retries.
        if spec.restart_policy.is_none() {
            spec.restart_policy = Some("Never".to_string());
        }

        // For Indexed mode, set hostname to {job-name}-{index} (K8s convention)
        if is_indexed {
            spec.hostname = Some(format!("{}-{}", job_name, index));
        }

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

        // Check ResourceQuota before creating pod
        super::check_resource_quota(&*self.storage, namespace).await?;

        let key = format!("/registry/pods/{}/{}", namespace, pod_name);
        self.storage.create(&key, &pod).await?;

        Ok(())
    }
}

/// Parse index ranges like "0,1,3-5" into a set of integers {0, 1, 3, 4, 5}
fn parse_index_ranges(s: &str) -> HashSet<i32> {
    let mut set = HashSet::new();
    if s.is_empty() {
        return set;
    }
    for part in s.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() == 2 {
                if let (Ok(start), Ok(end)) = (
                    bounds[0].trim().parse::<i32>(),
                    bounds[1].trim().parse::<i32>(),
                ) {
                    for i in start..=end {
                        set.insert(i);
                    }
                }
            }
        } else if let Ok(idx) = part.parse::<i32>() {
            set.insert(idx);
        }
    }
    set
}

/// Format a sorted, deduped list of indexes into compressed ranges: [0, 1, 2, 5] -> "0-2,5"
fn format_index_ranges(indexes: &[i32]) -> String {
    if indexes.is_empty() {
        return String::new();
    }
    let mut ranges: Vec<String> = Vec::new();
    let mut start = indexes[0];
    let mut end = indexes[0];
    for &idx in &indexes[1..] {
        if idx == end + 1 {
            end = idx;
        } else {
            if start == end {
                ranges.push(start.to_string());
            } else {
                ranges.push(format!("{}-{}", start, end));
            }
            start = idx;
            end = idx;
        }
    }
    if start == end {
        ranges.push(start.to_string());
    } else {
        ranges.push(format!("{}-{}", start, end));
    }
    ranges.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::workloads::{Job, JobSpec, PodTemplateSpec};
    use rusternetes_common::resources::{
        Container, ContainerState, ContainerStatus, Pod, PodSpec, PodStatus,
    };
    use rusternetes_common::types::{ObjectMeta, Phase, TypeMeta};
    use rusternetes_storage::MemoryStorage;
    use std::collections::HashMap;

    fn test_container() -> Container {
        Container {
            name: "test".to_string(),
            image: "busybox".to_string(),
            command: None,
            args: None,
            working_dir: None,
            ports: None,
            env: None,
            env_from: None,
            resources: None,
            volume_mounts: None,
            volume_devices: None,
            liveness_probe: None,
            readiness_probe: None,
            startup_probe: None,
            lifecycle: None,
            termination_message_path: None,
            termination_message_policy: None,
            image_pull_policy: None,
            security_context: None,
            stdin: None,
            stdin_once: None,
            tty: None,
            resize_policy: None,
            restart_policy: None,
        }
    }

    fn make_job(name: &str, namespace: &str, completions: i32, parallelism: i32) -> Job {
        Job {
            type_meta: TypeMeta {
                kind: "Job".to_string(),
                api_version: "batch/v1".to_string(),
            },
            metadata: ObjectMeta {
                name: name.to_string(),
                namespace: Some(namespace.to_string()),
                uid: "job-uid-1".to_string(),
                creation_timestamp: Some(chrono::Utc::now()),
                ..Default::default()
            },
            spec: JobSpec {
                template: PodTemplateSpec {
                    metadata: None,
                    spec: PodSpec {
                        containers: vec![test_container()],
                        ..Default::default()
                    },
                },
                completions: Some(completions),
                parallelism: Some(parallelism),
                backoff_limit: Some(6),
                active_deadline_seconds: None,
                selector: None,
                manual_selector: None,
                suspend: None,
                ttl_seconds_after_finished: None,
                completion_mode: None,
                backoff_limit_per_index: None,
                max_failed_indexes: None,
                pod_failure_policy: None,
                pod_replacement_policy: None,
                success_policy: None,
                managed_by: None,
            },
            status: None,
        }
    }

    fn make_pod(name: &str, namespace: &str, phase: Phase, job_name: &str, job_uid: &str) -> Pod {
        Pod {
            type_meta: TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta {
                name: name.to_string(),
                namespace: Some(namespace.to_string()),
                uid: format!("pod-uid-{}", name),
                labels: Some({
                    let mut m = HashMap::new();
                    m.insert("job-name".to_string(), job_name.to_string());
                    m
                }),
                owner_references: Some(vec![OwnerReference {
                    api_version: "batch/v1".to_string(),
                    kind: "Job".to_string(),
                    name: job_name.to_string(),
                    uid: job_uid.to_string(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]),
                creation_timestamp: Some(chrono::Utc::now()),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![test_container()],
                ..Default::default()
            }),
            status: Some(PodStatus {
                phase: Some(phase),
                ..Default::default()
            }),
        }
    }

    fn make_indexed_pod(
        name: &str,
        namespace: &str,
        phase: Phase,
        job_name: &str,
        job_uid: &str,
        index: i32,
    ) -> Pod {
        let mut pod = make_pod(name, namespace, phase, job_name, job_uid);
        pod.metadata.annotations = Some({
            let mut m = HashMap::new();
            m.insert(
                "batch.kubernetes.io/job-completion-index".to_string(),
                index.to_string(),
            );
            m
        });
        if let Some(ref mut labels) = pod.metadata.labels {
            labels.insert(
                "batch.kubernetes.io/job-completion-index".to_string(),
                index.to_string(),
            );
        }
        pod
    }

    fn make_failed_pod_with_exit_code(
        name: &str,
        namespace: &str,
        job_name: &str,
        job_uid: &str,
        index: i32,
        exit_code: i32,
    ) -> Pod {
        let mut pod = make_indexed_pod(name, namespace, Phase::Failed, job_name, job_uid, index);
        if let Some(ref mut status) = pod.status {
            status.container_statuses = Some(vec![ContainerStatus {
                name: "test".to_string(),
                ready: false,
                restart_count: 0,
                state: Some(ContainerState::Terminated {
                    exit_code,
                    signal: None,
                    reason: Some("Error".to_string()),
                    message: None,
                    started_at: None,
                    finished_at: None,
                    container_id: None,
                }),
                last_state: None,
                image: Some("busybox".to_string()),
                image_id: None,
                container_id: None,
                started: None,
                allocated_resources: None,
                allocated_resources_status: None,
                resources: None,
                volume_mounts: None,
                stop_signal: None,
                user: None,
            }]);
        }
        pod
    }

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

    #[test]
    fn test_parse_index_ranges() {
        assert_eq!(parse_index_ranges(""), HashSet::new());
        assert_eq!(
            parse_index_ranges("0"),
            [0].into_iter().collect::<HashSet<i32>>()
        );
        assert_eq!(
            parse_index_ranges("0,1,2"),
            [0, 1, 2].into_iter().collect::<HashSet<i32>>()
        );
        assert_eq!(
            parse_index_ranges("0-3"),
            [0, 1, 2, 3].into_iter().collect::<HashSet<i32>>()
        );
        assert_eq!(
            parse_index_ranges("0,2-4,7"),
            [0, 2, 3, 4, 7].into_iter().collect::<HashSet<i32>>()
        );
    }

    #[test]
    fn test_format_index_ranges() {
        assert_eq!(format_index_ranges(&[]), "");
        assert_eq!(format_index_ranges(&[0]), "0");
        assert_eq!(format_index_ranges(&[0, 1, 2]), "0-2");
        assert_eq!(format_index_ranges(&[0, 1, 2, 5]), "0-2,5");
        assert_eq!(format_index_ranges(&[0, 2, 3, 4, 7, 8]), "0,2-4,7-8");
    }

    #[tokio::test]
    async fn test_backoff_limit_per_index_tracks_failures() {
        let storage = Arc::new(MemoryStorage::new());

        // Create an indexed job with backoffLimitPerIndex=1, 3 completions
        let mut job = make_job("test-job", "default", 3, 3);
        job.spec.completion_mode = Some("Indexed".to_string());
        job.spec.backoff_limit_per_index = Some(1);
        job.spec.backoff_limit = Some(100); // high so global limit doesn't kick in

        let job_key = "/registry/jobs/default/test-job";
        storage.create(job_key, &job).await.unwrap();

        // Index 0 succeeded
        let pod0 = make_indexed_pod(
            "pod-0",
            "default",
            Phase::Succeeded,
            "test-job",
            "job-uid-1",
            0,
        );
        storage
            .create("/registry/pods/default/pod-0", &pod0)
            .await
            .unwrap();

        // Index 1 failed twice (exceeds backoffLimitPerIndex=1)
        let pod1a = make_indexed_pod(
            "pod-1a",
            "default",
            Phase::Failed,
            "test-job",
            "job-uid-1",
            1,
        );
        storage
            .create("/registry/pods/default/pod-1a", &pod1a)
            .await
            .unwrap();
        let pod1b = make_indexed_pod(
            "pod-1b",
            "default",
            Phase::Failed,
            "test-job",
            "job-uid-1",
            1,
        );
        storage
            .create("/registry/pods/default/pod-1b", &pod1b)
            .await
            .unwrap();

        // Index 2 succeeded
        let pod2 = make_indexed_pod(
            "pod-2",
            "default",
            Phase::Succeeded,
            "test-job",
            "job-uid-1",
            2,
        );
        storage
            .create("/registry/pods/default/pod-2", &pod2)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        // Re-read the job
        let updated_job: Job = storage.get(job_key).await.unwrap();
        let status = updated_job.status.unwrap();

        // Job should be failed because: succeeded(0,2) + failed(1) = 3 = completions
        // Index 1 exceeded backoffLimitPerIndex
        assert!(
            status
                .conditions
                .as_ref()
                .unwrap()
                .iter()
                .any(|c| c.condition_type == "Failed" && c.status == "True"),
            "Job should be marked as Failed"
        );

        // Failed indexes should contain index 1
        assert!(status.failed_indexes.is_some());
        let fi = parse_index_ranges(status.failed_indexes.as_deref().unwrap());
        assert!(fi.contains(&1), "Index 1 should be in failed_indexes");

        // Completed indexes should contain 0 and 2
        assert!(status.completed_indexes.is_some());
        let ci = parse_index_ranges(status.completed_indexes.as_deref().unwrap());
        assert!(
            ci.contains(&0) && ci.contains(&2),
            "Completed indexes should have 0 and 2"
        );
    }

    #[tokio::test]
    async fn test_backoff_limit_per_index_no_retry_for_exhausted_index() {
        let storage = Arc::new(MemoryStorage::new());

        // Create an indexed job with backoffLimitPerIndex=0, 3 completions
        let mut job = make_job("test-job2", "default", 3, 3);
        job.spec.completion_mode = Some("Indexed".to_string());
        job.spec.backoff_limit_per_index = Some(0);
        job.spec.backoff_limit = Some(100);

        let job_key = "/registry/jobs/default/test-job2";
        storage.create(job_key, &job).await.unwrap();

        // Index 0 failed once (exceeds limit of 0)
        let pod0 = make_indexed_pod(
            "pod-0",
            "default",
            Phase::Failed,
            "test-job2",
            "job-uid-1",
            0,
        );
        storage
            .create("/registry/pods/default/pod-0", &pod0)
            .await
            .unwrap();

        // Index 1 running
        let pod1 = make_indexed_pod(
            "pod-1",
            "default",
            Phase::Running,
            "test-job2",
            "job-uid-1",
            1,
        );
        storage
            .create("/registry/pods/default/pod-1", &pod1)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        // After reconciliation, no new pod should be created for index 0
        // Check that no new pod with index 0 was created
        let all_pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
        let index_0_pods: Vec<&Pod> = all_pods
            .iter()
            .filter(|p| {
                p.metadata
                    .annotations
                    .as_ref()
                    .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                    .map(|v| v == "0")
                    .unwrap_or(false)
            })
            .collect();

        // Should still be just the one failed pod for index 0, no retry
        assert_eq!(
            index_0_pods.len(),
            1,
            "Should not create a retry pod for exhausted index 0"
        );
    }

    #[tokio::test]
    async fn test_pod_failure_policy_fail_index() {
        let storage = Arc::new(MemoryStorage::new());

        let mut job = make_job("failindex-job", "default", 3, 3);
        job.spec.completion_mode = Some("Indexed".to_string());
        job.spec.backoff_limit = Some(100);
        // Set up a FailIndex policy for exit code 42
        job.spec.pod_failure_policy = Some(serde_json::json!({
            "rules": [
                {
                    "action": "FailIndex",
                    "onExitCodes": {
                        "operator": "In",
                        "values": [42]
                    }
                }
            ]
        }));

        let job_key = "/registry/jobs/default/failindex-job";
        storage.create(job_key, &job).await.unwrap();

        // Index 0 succeeded
        let pod0 = make_indexed_pod(
            "pod-0",
            "default",
            Phase::Succeeded,
            "failindex-job",
            "job-uid-1",
            0,
        );
        storage
            .create("/registry/pods/default/pod-0", &pod0)
            .await
            .unwrap();

        // Index 1 failed with exit code 42 -> should trigger FailIndex
        let pod1 =
            make_failed_pod_with_exit_code("pod-1", "default", "failindex-job", "job-uid-1", 1, 42);
        storage
            .create("/registry/pods/default/pod-1", &pod1)
            .await
            .unwrap();

        // Index 2 succeeded
        let pod2 = make_indexed_pod(
            "pod-2",
            "default",
            Phase::Succeeded,
            "failindex-job",
            "job-uid-1",
            2,
        );
        storage
            .create("/registry/pods/default/pod-2", &pod2)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        let updated_job: Job = storage.get(job_key).await.unwrap();
        let status = updated_job.status.unwrap();

        // Failed indexes should include index 1
        assert!(status.failed_indexes.is_some());
        let fi = parse_index_ranges(status.failed_indexes.as_deref().unwrap());
        assert!(
            fi.contains(&1),
            "Index 1 should be in failed_indexes due to FailIndex policy"
        );

        // No new pod should be created for index 1
        let all_pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
        let index_1_pods: Vec<&Pod> = all_pods
            .iter()
            .filter(|p| {
                p.metadata
                    .annotations
                    .as_ref()
                    .and_then(|a| a.get("batch.kubernetes.io/job-completion-index"))
                    .map(|v| v == "1")
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(
            index_1_pods.len(),
            1,
            "No retry pod for FailIndex-ed index 1"
        );
    }

    #[tokio::test]
    async fn test_pod_failure_policy_fail_job() {
        let storage = Arc::new(MemoryStorage::new());

        let mut job = make_job("failjob-job", "default", 3, 3);
        job.spec.backoff_limit = Some(100);
        job.spec.pod_failure_policy = Some(serde_json::json!({
            "rules": [
                {
                    "action": "FailJob",
                    "onExitCodes": {
                        "operator": "In",
                        "values": [99]
                    }
                }
            ]
        }));

        let job_key = "/registry/jobs/default/failjob-job";
        storage.create(job_key, &job).await.unwrap();

        // One pod failed with exit code 99 -> should trigger FailJob
        let pod = make_failed_pod_with_exit_code(
            "pod-fail",
            "default",
            "failjob-job",
            "job-uid-1",
            0,
            99,
        );
        storage
            .create("/registry/pods/default/pod-fail", &pod)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        let updated_job: Job = storage.get(job_key).await.unwrap();
        let status = updated_job.status.unwrap();

        assert!(
            status
                .conditions
                .as_ref()
                .unwrap()
                .iter()
                .any(|c| c.condition_type == "Failed"
                    && c.status == "True"
                    && c.reason.as_deref() == Some("PodFailurePolicy")),
            "Job should be Failed due to PodFailurePolicy"
        );
    }

    #[tokio::test]
    async fn test_local_restart_completion() {
        let storage = Arc::new(MemoryStorage::new());

        // Job with 2 completions, template has restartPolicy: OnFailure
        let mut job = make_job("restart-job", "default", 2, 2);
        job.spec.template.spec.restart_policy = Some("OnFailure".to_string());

        let job_key = "/registry/jobs/default/restart-job";
        storage.create(job_key, &job).await.unwrap();

        // Pod 1 succeeded (was restarted locally, restart_count > 0, now succeeded)
        let mut pod1 = make_pod(
            "pod-1",
            "default",
            Phase::Succeeded,
            "restart-job",
            "job-uid-1",
        );
        if let Some(ref mut status) = pod1.status {
            status.container_statuses = Some(vec![ContainerStatus {
                name: "test".to_string(),
                ready: false,
                restart_count: 3, // restarted 3 times before succeeding
                state: Some(ContainerState::Terminated {
                    exit_code: 0,
                    signal: None,
                    reason: Some("Completed".to_string()),
                    message: None,
                    started_at: None,
                    finished_at: None,
                    container_id: None,
                }),
                last_state: None,
                image: Some("busybox".to_string()),
                image_id: None,
                container_id: None,
                started: None,
                allocated_resources: None,
                allocated_resources_status: None,
                resources: None,
                volume_mounts: None,
                stop_signal: None,
                user: None,
            }]);
        }
        storage
            .create("/registry/pods/default/pod-1", &pod1)
            .await
            .unwrap();

        // Pod 2 succeeded
        let pod2 = make_pod(
            "pod-2",
            "default",
            Phase::Succeeded,
            "restart-job",
            "job-uid-1",
        );
        storage
            .create("/registry/pods/default/pod-2", &pod2)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        let updated_job: Job = storage.get(job_key).await.unwrap();
        let status = updated_job.status.unwrap();

        assert!(
            status
                .conditions
                .as_ref()
                .unwrap()
                .iter()
                .any(|c| c.condition_type == "Complete" && c.status == "True"),
            "Job should be Complete when locally restarted pods succeed"
        );
        assert_eq!(status.succeeded, Some(2));
    }

    #[tokio::test]
    async fn test_restart_policy_on_failure_preserved() {
        let storage = Arc::new(MemoryStorage::new());

        let mut job = make_job("onfailure-job", "default", 1, 1);
        job.spec.template.spec.restart_policy = Some("OnFailure".to_string());

        let job_key = "/registry/jobs/default/onfailure-job";
        storage.create(job_key, &job).await.unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        // Check that the created pod preserved the OnFailure restart policy
        let all_pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
        let job_pods: Vec<&Pod> = all_pods
            .iter()
            .filter(|p| {
                p.metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("job-name"))
                    .map(|v| v == "onfailure-job")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(job_pods.len(), 1);
        let restart_policy = job_pods[0].spec.as_ref().unwrap().restart_policy.as_deref();
        assert_eq!(
            restart_policy,
            Some("OnFailure"),
            "Pod should preserve OnFailure restart policy from template"
        );
    }

    #[tokio::test]
    async fn test_success_policy_succeeded_indexes() {
        let storage = Arc::new(MemoryStorage::new());

        // Indexed job with 5 completions, successPolicy requiring indexes 0-1
        let mut job = make_job("sp-job", "default", 5, 5);
        job.spec.completion_mode = Some("Indexed".to_string());
        job.spec.success_policy = Some(serde_json::json!({
            "rules": [
                {
                    "succeededIndexes": "0-1"
                }
            ]
        }));

        let job_key = "/registry/jobs/default/sp-job";
        storage.create(job_key, &job).await.unwrap();

        // Index 0 and 1 succeeded
        let pod0 = make_indexed_pod(
            "pod-0",
            "default",
            Phase::Succeeded,
            "sp-job",
            "job-uid-1",
            0,
        );
        storage
            .create("/registry/pods/default/pod-0", &pod0)
            .await
            .unwrap();
        let pod1 = make_indexed_pod(
            "pod-1",
            "default",
            Phase::Succeeded,
            "sp-job",
            "job-uid-1",
            1,
        );
        storage
            .create("/registry/pods/default/pod-1", &pod1)
            .await
            .unwrap();

        // Indexes 2-4 are still pending
        let pod2 = make_indexed_pod("pod-2", "default", Phase::Pending, "sp-job", "job-uid-1", 2);
        storage
            .create("/registry/pods/default/pod-2", &pod2)
            .await
            .unwrap();
        let pod3 = make_indexed_pod("pod-3", "default", Phase::Pending, "sp-job", "job-uid-1", 3);
        storage
            .create("/registry/pods/default/pod-3", &pod3)
            .await
            .unwrap();
        let pod4 = make_indexed_pod("pod-4", "default", Phase::Pending, "sp-job", "job-uid-1", 4);
        storage
            .create("/registry/pods/default/pod-4", &pod4)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        let updated_job: Job = storage.get(job_key).await.unwrap();
        let status = updated_job.status.unwrap();

        // Job should be complete via successPolicy even though indexes 2-4 are pending
        assert!(
            status
                .conditions
                .as_ref()
                .unwrap()
                .iter()
                .any(|c| c.condition_type == "Complete" && c.status == "True"),
            "Job should be Complete via successPolicy"
        );
        assert!(
            status
                .conditions
                .as_ref()
                .unwrap()
                .iter()
                .any(|c| c.condition_type == "SuccessCriteriaMet" && c.status == "True"),
            "SuccessCriteriaMet condition should be set"
        );
        assert!(status.completion_time.is_some());
    }

    #[tokio::test]
    async fn test_success_policy_succeeded_count() {
        let storage = Arc::new(MemoryStorage::new());

        let mut job = make_job("sp-count-job", "default", 5, 5);
        job.spec.completion_mode = Some("Indexed".to_string());
        job.spec.success_policy = Some(serde_json::json!({
            "rules": [
                {
                    "succeededCount": 2
                }
            ]
        }));

        let job_key = "/registry/jobs/default/sp-count-job";
        storage.create(job_key, &job).await.unwrap();

        // 2 indexes succeeded
        let pod0 = make_indexed_pod(
            "pod-0",
            "default",
            Phase::Succeeded,
            "sp-count-job",
            "job-uid-1",
            0,
        );
        storage
            .create("/registry/pods/default/pod-0", &pod0)
            .await
            .unwrap();
        let pod1 = make_indexed_pod(
            "pod-1",
            "default",
            Phase::Succeeded,
            "sp-count-job",
            "job-uid-1",
            1,
        );
        storage
            .create("/registry/pods/default/pod-1", &pod1)
            .await
            .unwrap();

        // Others still pending
        let pod2 = make_indexed_pod(
            "pod-2",
            "default",
            Phase::Pending,
            "sp-count-job",
            "job-uid-1",
            2,
        );
        storage
            .create("/registry/pods/default/pod-2", &pod2)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        let updated_job: Job = storage.get(job_key).await.unwrap();
        let status = updated_job.status.unwrap();

        assert!(
            status
                .conditions
                .as_ref()
                .unwrap()
                .iter()
                .any(|c| c.condition_type == "Complete" && c.status == "True"),
            "Job should be Complete via succeededCount successPolicy"
        );
    }

    #[tokio::test]
    async fn test_success_policy_not_met_yet() {
        let storage = Arc::new(MemoryStorage::new());

        let mut job = make_job("sp-notyet", "default", 5, 5);
        job.spec.completion_mode = Some("Indexed".to_string());
        job.spec.success_policy = Some(serde_json::json!({
            "rules": [
                {
                    "succeededIndexes": "0-2"
                }
            ]
        }));

        let job_key = "/registry/jobs/default/sp-notyet";
        storage.create(job_key, &job).await.unwrap();

        // Only index 0 succeeded — need 0, 1, 2
        let pod0 = make_indexed_pod(
            "pod-0",
            "default",
            Phase::Succeeded,
            "sp-notyet",
            "job-uid-1",
            0,
        );
        storage
            .create("/registry/pods/default/pod-0", &pod0)
            .await
            .unwrap();
        let pod1 = make_indexed_pod(
            "pod-1",
            "default",
            Phase::Running,
            "sp-notyet",
            "job-uid-1",
            1,
        );
        storage
            .create("/registry/pods/default/pod-1", &pod1)
            .await
            .unwrap();

        let controller = JobController::new(storage.clone());
        let mut job: Job = storage.get(job_key).await.unwrap();
        controller.reconcile(&mut job).await.unwrap();

        let updated_job: Job = storage.get(job_key).await.unwrap();
        let status = updated_job.status.unwrap();

        // Should NOT be complete yet
        let is_complete = status
            .conditions
            .as_ref()
            .map(|c| {
                c.iter()
                    .any(|cond| cond.condition_type == "Complete" && cond.status == "True")
            })
            .unwrap_or(false);
        assert!(
            !is_complete,
            "Job should NOT be complete when not all required indexes succeeded"
        );
    }
}

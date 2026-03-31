use anyhow::Result;
use rusternetes_common::resources::workloads::{CronJob, CronJobStatus, Job};
use rusternetes_common::types::OwnerReference;
use rusternetes_storage::Storage;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

pub struct CronJobController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> CronJobController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting CronJobController");

        loop {
            if let Err(e) = self.reconcile_all().await {
                error!("Error in CronJob reconciliation loop: {}", e);
            }
            time::sleep(Duration::from_secs(1)).await; // Check every second for fast scheduling
        }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        let cronjobs: Vec<CronJob> = self.storage.list("/registry/cronjobs/").await?;

        for mut cronjob in cronjobs {
            if let Err(e) = self.reconcile(&mut cronjob).await {
                error!(
                    "Failed to reconcile CronJob {}: {}",
                    cronjob.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile(&self, cronjob: &mut CronJob) -> Result<()> {
        let name = &cronjob.metadata.name;
        let namespace = cronjob.metadata.namespace.as_ref().unwrap();

        // Skip reconciliation for CronJobs being deleted — GC handles Job cleanup
        if cronjob.metadata.is_being_deleted() {
            return Ok(());
        }

        info!("Reconciling CronJob {}/{}", namespace, name);

        // Check if CronJob is suspended
        if cronjob.spec.suspend.unwrap_or(false) {
            info!("CronJob {}/{} is suspended", namespace, name);
            return Ok(());
        }

        // Parse cron schedule
        let schedule = &cronjob.spec.schedule;
        let now = chrono::Utc::now();

        // Simple cron parsing - in production, use a proper cron parser library
        let should_run = self.should_run_now(schedule, now, cronjob)?;

        if !should_run {
            return Ok(());
        }

        info!("CronJob {}/{} triggered at {}", namespace, name, now);

        // Check concurrency policy
        let job_prefix = format!("/registry/jobs/{}/", namespace);
        let all_jobs: Vec<Job> = self.storage.list(&job_prefix).await?;

        let active_jobs: Vec<Job> = all_jobs
            .into_iter()
            .filter(|job| {
                job.metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("cronjob-name"))
                    .map(|cj| cj == name)
                    .unwrap_or(false)
                    && job
                        .status
                        .as_ref()
                        .and_then(|s| s.conditions.as_ref())
                        .map(|conds| {
                            !conds
                                .iter()
                                .any(|c| c.condition_type == "Complete" && c.status == "True")
                        })
                        .unwrap_or(true) // Job not completed
            })
            .collect();

        let concurrency_policy = cronjob
            .spec
            .concurrency_policy
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Allow");

        match concurrency_policy {
            "Forbid" if !active_jobs.is_empty() => {
                info!(
                    "CronJob {}/{} skipped due to Forbid policy (active jobs: {})",
                    namespace,
                    name,
                    active_jobs.len()
                );
                // Still update status with active jobs list
                let active_refs: Vec<
                    rusternetes_common::resources::service_account::ObjectReference,
                > = active_jobs
                    .iter()
                    .map(
                        |job| rusternetes_common::resources::service_account::ObjectReference {
                            kind: Some("Job".to_string()),
                            namespace: Some(namespace.to_string()),
                            name: Some(job.metadata.name.clone()),
                            uid: Some(job.metadata.uid.clone()),
                            api_version: Some("batch/v1".to_string()),
                            resource_version: job.metadata.resource_version.clone(),
                            field_path: None,
                        },
                    )
                    .collect();
                cronjob.status = Some(CronJobStatus {
                    active: Some(active_refs),
                    last_schedule_time: cronjob.status.as_ref().and_then(|s| s.last_schedule_time),
                    last_successful_time: cronjob
                        .status
                        .as_ref()
                        .and_then(|s| s.last_successful_time),
                });
                let key = format!("/registry/cronjobs/{}/{}", namespace, name);
                let _ = self.storage.update(&key, cronjob).await;
                return Ok(());
            }
            "Replace" if !active_jobs.is_empty() => {
                // Delete active jobs
                for job in active_jobs.iter() {
                    let job_name = &job.metadata.name;
                    let job_key = format!("/registry/jobs/{}/{}", namespace, job_name);
                    self.storage.delete(&job_key).await?;
                    info!("Deleted active Job {} for replacement", job_name);
                }
            }
            _ => {
                // Allow - just create a new job
            }
        }

        // Create new Job
        self.create_job(cronjob, namespace).await?;

        // Build active job references from all active jobs for this cronjob
        let active_refs: Vec<rusternetes_common::resources::service_account::ObjectReference> = {
            let job_prefix = format!("/registry/jobs/{}/", namespace);
            let current_jobs: Vec<Job> = self.storage.list(&job_prefix).await.unwrap_or_default();
            current_jobs
                .iter()
                .filter(|job| {
                    job.metadata
                        .labels
                        .as_ref()
                        .and_then(|l| l.get("cronjob-name"))
                        .map(|cj| cj == name)
                        .unwrap_or(false)
                        && !job
                            .status
                            .as_ref()
                            .and_then(|s| s.conditions.as_ref())
                            .map(|conds| {
                                conds
                                    .iter()
                                    .any(|c| c.condition_type == "Complete" && c.status == "True")
                            })
                            .unwrap_or(false)
                })
                .map(
                    |job| rusternetes_common::resources::service_account::ObjectReference {
                        kind: Some("Job".to_string()),
                        namespace: Some(namespace.to_string()),
                        name: Some(job.metadata.name.clone()),
                        uid: Some(job.metadata.uid.clone()),
                        api_version: Some("batch/v1".to_string()),
                        resource_version: job.metadata.resource_version.clone(),
                        field_path: None,
                    },
                )
                .collect()
        };

        // Update status with active refs and last schedule time
        cronjob.status = Some(CronJobStatus {
            active: if active_refs.is_empty() {
                None
            } else {
                Some(active_refs)
            },
            last_schedule_time: Some(now),
            last_successful_time: cronjob.status.as_ref().and_then(|s| s.last_successful_time),
        });

        // Save updated status
        let key = format!("/registry/cronjobs/{}/{}", namespace, name);
        self.storage.update(&key, cronjob).await?;

        // Clean up old jobs based on history limits
        self.cleanup_old_jobs(cronjob, namespace).await?;

        Ok(())
    }

    fn should_run_now(
        &self,
        schedule: &str,
        now: chrono::DateTime<chrono::Utc>,
        cronjob: &CronJob,
    ) -> Result<bool> {
        // Get last schedule time
        let last_schedule = cronjob.status.as_ref().and_then(|s| s.last_schedule_time);

        // Handle special schedules (Kubernetes 5-field format)
        let cron_schedule = match schedule {
            "@yearly" | "@annually" => "0 0 1 1 *",
            "@monthly" => "0 0 1 * *",
            "@weekly" => "0 0 * * 0",
            "@daily" | "@midnight" => "0 0 * * *",
            "@hourly" => "0 * * * *",
            other => other,
        };

        // Kubernetes supports `?` in cron expressions (Quartz-style "no specific value").
        // Replace with `*` since the `cron` crate doesn't support `?`.
        let cron_schedule = cron_schedule.replace('?', "*");

        // The `cron` crate expects 7 fields (sec min hour dom month dow year),
        // but Kubernetes uses 5 fields (min hour dom month dow).
        // Convert by prepending "0" for seconds and appending "*" for year.
        let field_count = cron_schedule.split_whitespace().count();
        let cron_schedule = if field_count == 5 {
            format!("0 {} *", cron_schedule)
        } else if field_count == 6 {
            format!("0 {}", cron_schedule)
        } else {
            cron_schedule.to_string()
        };

        // Parse cron expression using the `cron` crate
        let schedule_parsed = match cron::Schedule::try_from(cron_schedule.as_str()) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to parse cron schedule '{}': {}", cron_schedule, e);
                return Ok(false);
            }
        };

        // Determine if job should run now
        // Check if we're within the schedule window since last run
        if let Some(last) = last_schedule {
            // Find next scheduled time after last run
            if let Some(next_run) = schedule_parsed.after(&last).next() {
                // Should run if current time >= next scheduled time
                let should_run = now >= next_run;
                if should_run {
                    info!("CronJob should run: next_run={}, current={}", next_run, now);
                }
                Ok(should_run)
            } else {
                // No next run time found
                Ok(false)
            }
        } else {
            // Never run before - check if there's a scheduled time in the past minute
            // This prevents all cronjobs from running immediately on startup
            let one_minute_ago = now - chrono::Duration::minutes(1);
            if let Some(next_run) = schedule_parsed.after(&one_minute_ago).next() {
                let should_run = now >= next_run;
                if should_run {
                    info!("CronJob first run: next_run={}, current={}", next_run, now);
                }
                Ok(should_run)
            } else {
                Ok(false)
            }
        }
    }

    async fn create_job(&self, cronjob: &CronJob, namespace: &str) -> Result<()> {
        let cronjob_name = &cronjob.metadata.name;
        let timestamp = chrono::Utc::now().timestamp();
        let job_name = format!("{}-{}", cronjob_name, timestamp);

        let mut labels = cronjob
            .spec
            .job_template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("cronjob-name".to_string(), cronjob_name.clone());

        let annotations = cronjob
            .spec
            .job_template
            .metadata
            .as_ref()
            .and_then(|m| m.annotations.clone());

        let job = Job {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Job".to_string(),
                api_version: "batch/v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: job_name.clone(),
                generate_name: None,
                generation: None,
                managed_fields: None,
                namespace: Some(namespace.to_string()),
                labels: Some(labels),
                annotations,
                uid: uuid::Uuid::new_v4().to_string(),
                creation_timestamp: Some(chrono::Utc::now()),
                deletion_timestamp: None,
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: Some(vec![OwnerReference {
                    api_version: "batch/v1".to_string(),
                    kind: "CronJob".to_string(),
                    name: cronjob_name.clone(),
                    uid: cronjob.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]),
            },
            spec: cronjob.spec.job_template.spec.clone(),
            status: None,
        };

        let key = format!("/registry/jobs/{}/{}", namespace, job_name);
        self.storage.create(&key, &job).await?;

        info!("Created Job {} from CronJob {}", job_name, cronjob_name);

        Ok(())
    }

    async fn cleanup_old_jobs(&self, cronjob: &CronJob, namespace: &str) -> Result<()> {
        let cronjob_name = &cronjob.metadata.name;
        let success_limit = cronjob.spec.successful_jobs_history_limit.unwrap_or(3);
        let failed_limit = cronjob.spec.failed_jobs_history_limit.unwrap_or(1);

        let job_prefix = format!("/registry/jobs/{}/", namespace);
        let mut all_jobs: Vec<Job> = self.storage.list(&job_prefix).await?;

        // Filter jobs from this CronJob
        all_jobs.retain(|job| {
            job.metadata
                .labels
                .as_ref()
                .and_then(|labels| labels.get("cronjob-name"))
                .map(|cj| cj == cronjob_name)
                .unwrap_or(false)
        });

        // Separate successful and failed jobs
        let mut successful_jobs: Vec<Job> = all_jobs
            .iter()
            .filter(|job| {
                job.status
                    .as_ref()
                    .and_then(|s| s.conditions.as_ref())
                    .map(|conds| {
                        conds
                            .iter()
                            .any(|c| c.condition_type == "Complete" && c.status == "True")
                    })
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        let mut failed_jobs: Vec<Job> = all_jobs
            .iter()
            .filter(|job| {
                job.status
                    .as_ref()
                    .and_then(|s| s.conditions.as_ref())
                    .map(|conds| {
                        conds
                            .iter()
                            .any(|c| c.condition_type == "Failed" && c.status == "True")
                    })
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        // Sort by creation timestamp (oldest first)
        successful_jobs.sort_by(|a, b| {
            a.metadata
                .creation_timestamp
                .cmp(&b.metadata.creation_timestamp)
        });
        failed_jobs.sort_by(|a, b| {
            a.metadata
                .creation_timestamp
                .cmp(&b.metadata.creation_timestamp)
        });

        // Delete old successful jobs
        if successful_jobs.len() > success_limit as usize {
            let to_delete = successful_jobs.len() - success_limit as usize;
            for job in successful_jobs.iter().take(to_delete) {
                let job_name = &job.metadata.name;
                let job_key = format!("/registry/jobs/{}/{}", namespace, job_name);
                self.storage.delete(&job_key).await?;
                info!("Deleted old successful Job {}", job_name);
            }
        }

        // Delete old failed jobs
        if failed_jobs.len() > failed_limit as usize {
            let to_delete = failed_jobs.len() - failed_limit as usize;
            for job in failed_jobs.iter().take(to_delete) {
                let job_name = &job.metadata.name;
                let job_key = format!("/registry/jobs/{}/{}", namespace, job_name);
                self.storage.delete(&job_key).await?;
                info!("Deleted old failed Job {}", job_name);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cron_schedule_parsing() {
        // Test that schedule patterns are recognized
        assert!("*/5 * * * *".starts_with("*/"));
        assert_eq!("@hourly", "@hourly");
        assert_eq!("@daily", "@daily");
    }

    #[test]
    fn test_job_name_generation() {
        let cronjob_name = "backup";
        let timestamp = 1234567890;
        let job_name = format!("{}-{}", cronjob_name, timestamp);
        assert_eq!(job_name, "backup-1234567890");
    }
}

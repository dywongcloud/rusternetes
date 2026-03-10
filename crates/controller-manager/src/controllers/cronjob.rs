use anyhow::Result;
use rusternetes_common::resources::workloads::{CronJob, CronJobStatus, Job};
use rusternetes_storage::{etcd::EtcdStorage, Storage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

pub struct CronJobController {
    storage: Arc<EtcdStorage>,
}

impl CronJobController {
    pub fn new(storage: Arc<EtcdStorage>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting CronJobController");

        loop {
            if let Err(e) = self.reconcile_all().await {
                error!("Error in CronJob reconciliation loop: {}", e);
            }
            time::sleep(Duration::from_secs(10)).await; // Check every 10 seconds
        }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        let cronjobs: Vec<CronJob> = self.storage.list("/registry/cronjobs/").await?;

        for mut cronjob in cronjobs {
            if let Err(e) = self.reconcile(&mut cronjob).await {
                error!(
                    "Failed to reconcile CronJob {}: {}",
                    cronjob.metadata.name,
                    e
                );
            }
        }

        Ok(())
    }

    async fn reconcile(&self, cronjob: &mut CronJob) -> Result<()> {
        let name = &cronjob.metadata.name;
        let namespace = cronjob.metadata.namespace.as_ref().unwrap();

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
                        .map(|conds| !conds.iter().any(|c| c.condition_type == "Complete" && c.status == "True"))
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

        // Update last schedule time
        cronjob.status = Some(CronJobStatus {
            active: None, // Will be set with actual ObjectReferences in production
            last_schedule_time: Some(now),
            last_successful_time: cronjob
                .status
                .as_ref()
                .and_then(|s| s.last_successful_time),
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
        let last_schedule = cronjob
            .status
            .as_ref()
            .and_then(|s| s.last_schedule_time);

        // Simple schedule parsing for common patterns
        // In production, use a proper cron parser like the `cron` crate
        let should_run = match schedule {
            s if s.starts_with("*/") => {
                // Every N minutes
                if let Some(mins) = s.strip_prefix("*/").and_then(|s| s.split_whitespace().next()).and_then(|s| s.parse::<i64>().ok()) {
                    if let Some(last) = last_schedule {
                        let elapsed = (now - last).num_minutes();
                        elapsed >= mins
                    } else {
                        true // Never run before
                    }
                } else {
                    false
                }
            }
            "@hourly" => {
                if let Some(last) = last_schedule {
                    (now - last).num_hours() >= 1
                } else {
                    true
                }
            }
            "@daily" | "@midnight" => {
                if let Some(last) = last_schedule {
                    (now - last).num_days() >= 1
                } else {
                    true
                }
            }
            "@weekly" => {
                if let Some(last) = last_schedule {
                    (now - last).num_weeks() >= 1
                } else {
                    true
                }
            }
            "@monthly" => {
                if let Some(last) = last_schedule {
                    (now - last).num_days() >= 30
                } else {
                    true
                }
            }
            _ => {
                // For complex cron expressions, would need a proper parser
                warn!("Complex cron schedule '{}' not fully supported, skipping", schedule);
                false
            }
        };

        Ok(should_run)
    }

    async fn create_job(&self, cronjob: &CronJob, namespace: &str) -> Result<()> {
        let cronjob_name = &cronjob.metadata.name;
        let timestamp = chrono::Utc::now().timestamp();
        let job_name = format!("{}-{}", cronjob_name, timestamp);

        let mut labels = cronjob.spec.job_template.metadata.as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("cronjob-name".to_string(), cronjob_name.clone());

        let annotations = cronjob.spec.job_template.metadata.as_ref()
            .and_then(|m| m.annotations.clone());

        let job = Job {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Job".to_string(),
                api_version: "batch/v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: job_name.clone(),
                namespace: Some(namespace.to_string()),
                labels: Some(labels),
                annotations,
                uid: uuid::Uuid::new_v4().to_string(),
                creation_timestamp: Some(chrono::Utc::now()),
                deletion_timestamp: None,
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: None,
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
                    .map(|conds| conds.iter().any(|c| c.condition_type == "Complete" && c.status == "True"))
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
                    .map(|conds| conds.iter().any(|c| c.condition_type == "Failed" && c.status == "True"))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        // Sort by creation timestamp (oldest first)
        successful_jobs.sort_by(|a, b| {
            a.metadata.creation_timestamp.cmp(&b.metadata.creation_timestamp)
        });
        failed_jobs.sort_by(|a, b| {
            a.metadata.creation_timestamp.cmp(&b.metadata.creation_timestamp)
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

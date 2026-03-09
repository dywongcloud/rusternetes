use crate::resources::pod::PodSpec;
use crate::types::{LabelSelector, ObjectMeta, TypeMeta};
use serde::{Deserialize, Serialize};

/// StatefulSet represents a set of pods with consistent identities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatefulSet {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    pub spec: StatefulSetSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatefulSetStatus>,
}

impl StatefulSet {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>, spec: StatefulSetSpec) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "StatefulSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            spec,
            status: None,
        }
    }
}

/// StatefulSetSpec defines the desired state of a StatefulSet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatefulSetSpec {
    /// Number of desired pods
    pub replicas: i32,

    /// Selector for pods
    pub selector: LabelSelector,

    /// Service name for network identity
    pub service_name: String,

    /// Template for pod creation
    pub template: PodTemplateSpec,

    /// Update strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_strategy: Option<StatefulSetUpdateStrategy>,

    /// Pod management policy: OrderedReady or Parallel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_management_policy: Option<String>,
}

/// StatefulSetUpdateStrategy indicates the strategy for updating StatefulSet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatefulSetUpdateStrategy {
    /// Type of update strategy: RollingUpdate or OnDelete
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub strategy_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolling_update: Option<RollingUpdateStatefulSetStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollingUpdateStatefulSetStrategy {
    /// The maximum number of pods that can be unavailable during the update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<i32>,
}

/// StatefulSetStatus represents the current state of a StatefulSet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatefulSetStatus {
    /// Number of replicas
    pub replicas: i32,

    /// Number of ready replicas
    pub ready_replicas: i32,

    /// Number of current replicas
    pub current_replicas: i32,

    /// Number of updated replicas
    pub updated_replicas: i32,
}

/// DaemonSet ensures that all (or some) nodes run a copy of a pod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSet {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    pub spec: DaemonSetSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DaemonSetStatus>,
}

impl DaemonSet {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>, spec: DaemonSetSpec) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "DaemonSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            spec,
            status: None,
        }
    }
}

/// DaemonSetSpec defines the desired state of a DaemonSet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSetSpec {
    /// Selector for pods
    pub selector: LabelSelector,

    /// Template for pod creation
    pub template: PodTemplateSpec,

    /// Update strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_strategy: Option<DaemonSetUpdateStrategy>,
}

/// DaemonSetUpdateStrategy indicates the strategy for updating DaemonSet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonSetUpdateStrategy {
    /// Type of update strategy: RollingUpdate or OnDelete
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub strategy_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolling_update: Option<RollingUpdateDaemonSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollingUpdateDaemonSet {
    /// The maximum number of pods that can be unavailable during the update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_unavailable: Option<String>, // IntOrString
}

/// DaemonSetStatus represents the current state of a DaemonSet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonSetStatus {
    /// Number of nodes that should be running the daemon pod
    pub desired_number_scheduled: i32,

    /// Number of nodes running at least one daemon pod
    pub current_number_scheduled: i32,

    /// Number of nodes with ready daemon pods
    pub number_ready: i32,

    /// Number of nodes that should be running but aren't
    pub number_misscheduled: i32,
}

/// Job represents a single batch process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    pub spec: JobSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<JobStatus>,
}

impl Job {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>, spec: JobSpec) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "Job".to_string(),
                api_version: "batch/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            spec,
            status: None,
        }
    }
}

/// JobSpec defines the desired state of a Job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Template for pod creation
    pub template: PodTemplateSpec,

    /// Number of successful completions required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<i32>,

    /// Maximum number of pods that can run in parallel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallelism: Option<i32>,

    /// Number of retries before marking the job as failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_limit: Option<i32>,

    /// Duration in seconds relative to startTime that the job may be active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_deadline_seconds: Option<i64>,
}

/// JobStatus represents the current state of a Job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobStatus {
    /// Number of actively running pods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<i32>,

    /// Number of successfully completed pods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<i32>,

    /// Number of failed pods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<i32>,

    /// Conditions of the job
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<JobCondition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobCondition {
    /// Type of condition: Complete or Failed
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Status of the condition: True, False, Unknown
    pub status: String,

    /// Last time the condition was probed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Last time the condition transitioned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_time: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// CronJob manages time-based jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    pub spec: CronJobSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CronJobStatus>,
}

impl CronJob {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>, spec: CronJobSpec) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "CronJob".to_string(),
                api_version: "batch/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            spec,
            status: None,
        }
    }
}

/// CronJobSpec defines the desired state of a CronJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobSpec {
    /// Cron schedule (e.g., "0 * * * *")
    pub schedule: String,

    /// Job template
    pub job_template: JobTemplateSpec,

    /// Concurrency policy: Allow, Forbid, or Replace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_policy: Option<String>,

    /// Whether the cron job is suspended
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend: Option<bool>,

    /// Number of successful job history to retain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successful_jobs_history_limit: Option<i32>,

    /// Number of failed job history to retain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_jobs_history_limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobTemplateSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ObjectMeta>,

    pub spec: JobSpec,
}

/// CronJobStatus represents the current state of a CronJob
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CronJobStatus {
    /// List of currently running jobs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<Vec<crate::resources::service_account::ObjectReference>>,

    /// Last time the job was scheduled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_schedule_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Last successful job time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_successful_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// PodTemplateSpec describes the pod that will be created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodTemplateSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ObjectMeta>,

    pub spec: PodSpec,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let template = PodTemplateSpec {
            metadata: None,
            spec: PodSpec {
                containers: vec![],
                volumes: None,
                restart_policy: Some("Never".to_string()),
                node_name: None,
                node_selector: None,
                service_account_name: None,
                hostname: None,
                host_network: None,
                affinity: None,
                tolerations: None,
                priority: None,
                priority_class_name: None,
            },
        };

        let job_spec = JobSpec {
            template,
            completions: Some(1),
            parallelism: Some(1),
            backoff_limit: Some(3),
            active_deadline_seconds: None,
        };

        let job = Job::new("test-job", "default", job_spec);

        assert_eq!(job.metadata.name, "test-job");
        assert_eq!(job.type_meta.api_version, "batch/v1");
    }
}

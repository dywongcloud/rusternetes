use chrono::Utc;
use rusternetes_common::resources::{Event, EventSource, EventType, ObjectReference, Pod};
use rusternetes_storage::Storage;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// EventsController creates events for pod lifecycle changes
pub struct EventsController<S: Storage> {
    storage: Arc<S>,
    sync_interval: Duration,
}

impl<S: Storage> EventsController<S> {
    pub fn new(storage: Arc<S>, sync_interval_secs: u64) -> Self {
        Self {
            storage,
            sync_interval: Duration::from_secs(sync_interval_secs),
        }
    }

    /// Start the controller loop
    pub async fn run(self: Arc<Self>) {
        println!("Starting Events controller (sync interval: {:?})", self.sync_interval);
        loop {
            if let Err(e) = self.reconcile_all().await {
                eprintln!("Error in events controller reconciliation: {}", e);
            }
            sleep(self.sync_interval).await;
        }
    }

    /// Reconcile all pods and create events for lifecycle changes
    pub async fn reconcile_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get all pods
        let pods: Vec<Pod> = self.storage.list("/registry/pods/").await?;

        for pod in pods {
            if let Err(e) = self.reconcile_pod_events(&pod).await {
                eprintln!(
                    "Error reconciling events for pod {}: {}",
                    &pod.metadata.name,
                    e
                );
            }
        }

        // Clean up old events (older than 1 hour)
        self.cleanup_old_events().await?;

        Ok(())
    }

    /// Reconcile events for a single pod
    async fn reconcile_pod_events(&self, pod: &Pod) -> Result<(), Box<dyn std::error::Error>> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
        let pod_uid = &pod.metadata.uid;

        // Create object reference for the pod
        let pod_ref = ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some(namespace.to_string()),
            name: Some(pod_name.to_string()),
            uid: Some(pod_uid.to_string()),
            api_version: Some("v1".to_string()),
            resource_version: pod.metadata.resource_version.clone(),
            field_path: None,
        };

        // Check pod phase and create appropriate events
        if let Some(status) = &pod.status {
            use rusternetes_common::types::Phase;

            match &status.phase {
                Some(Phase::Pending) => {
                    // Check if pod is scheduled
                    if let Some(spec) = &pod.spec {
                        if spec.node_name.is_some() {
                            self.create_event_if_new(
                                namespace,
                                &pod_ref,
                                "Scheduled",
                                &format!("Successfully assigned {} to {}", pod_name, spec.node_name.as_deref().unwrap_or("node")),
                                EventType::Normal,
                                Some("scheduler".to_string()),
                            ).await?;
                        }
                    }
                }
                Some(Phase::Running) => {
                    self.create_event_if_new(
                        namespace,
                        &pod_ref,
                        "Started",
                        &format!("Started pod {}", pod_name),
                        EventType::Normal,
                        Some("kubelet".to_string()),
                    ).await?;

                    // Create events for container statuses
                    if let Some(container_statuses) = &status.container_statuses {
                        for container_status in container_statuses {
                            // Check for running containers
                            if matches!(&container_status.state, Some(state) if matches!(state, rusternetes_common::resources::ContainerState::Running { .. })) {
                                self.create_event_if_new(
                                    namespace,
                                    &pod_ref,
                                    "Pulled",
                                    &format!("Container image \\\"{}\\\" already present on machine", container_status.image.as_deref().unwrap_or("unknown")),
                                    EventType::Normal,
                                    Some("kubelet".to_string()),
                                ).await?;

                                self.create_event_if_new(
                                    namespace,
                                    &pod_ref,
                                    "Created",
                                    &format!("Created container {}", container_status.name),
                                    EventType::Normal,
                                    Some("kubelet".to_string()),
                                ).await?;

                                self.create_event_if_new(
                                    namespace,
                                    &pod_ref,
                                    "Started",
                                    &format!("Started container {}", container_status.name),
                                    EventType::Normal,
                                    Some("kubelet".to_string()),
                                ).await?;
                            }

                            // Check for restarts
                            if container_status.restart_count > 0 {
                                self.create_event_if_new(
                                    namespace,
                                    &pod_ref,
                                    "BackOff",
                                    &format!("Back-off restarting failed container {} in pod {}", container_status.name, pod_name),
                                    EventType::Warning,
                                    Some("kubelet".to_string()),
                                ).await?;
                            }
                        }
                    }
                }
                Some(Phase::Succeeded) => {
                    self.create_event_if_new(
                        namespace,
                        &pod_ref,
                        "Completed",
                        &format!("Pod {} completed successfully", pod_name),
                        EventType::Normal,
                        Some("kubelet".to_string()),
                    ).await?;
                }
                Some(Phase::Failed) => {
                    let reason = status.reason.as_deref().unwrap_or("Unknown");
                    let message = status.message.as_deref().unwrap_or("Pod failed");
                    self.create_event_if_new(
                        namespace,
                        &pod_ref,
                        "Failed",
                        &format!("Pod {} failed: {} - {}", pod_name, reason, message),
                        EventType::Warning,
                        Some("kubelet".to_string()),
                    ).await?;
                }
                Some(Phase::Unknown) => {}
                Some(Phase::Active) => {
                    // Active phase for namespaces - not relevant for pod events
                }
                None => {
                    // Pod has no phase set
                }
            }
        }

        Ok(())
    }

    /// Create an event if it doesn't already exist (deduplicate)
    async fn create_event_if_new(
        &self,
        namespace: &str,
        involved_object: &ObjectReference,
        reason: &str,
        message: &str,
        event_type: EventType,
        component: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Generate event name based on involved object and reason
        let event_name = Event::generate_name(involved_object, reason);

        // Check if event already exists
        let key = format!("/registry/events/{}/{}", namespace, event_name);
        if let Ok(existing_event) = self.storage.get::<Event>(&key).await {
            // Update count and last timestamp
            let mut updated_event = existing_event;
            updated_event.count += 1;
            updated_event.last_timestamp = Utc::now();
            self.storage.update(&key, &updated_event).await?;
            return Ok(());
        }

        // Create new event
        let mut event = Event::new(
            event_name.clone(),
            namespace.to_string(),
            involved_object.clone(),
            reason.to_string(),
            message.to_string(),
            event_type,
        );

        // Set component if provided
        if let Some(comp) = component {
            event.source = EventSource {
                component: comp,
                host: None,
            };
        }

        // Store event
        self.storage.create(&key, &event).await?;

        println!(
            "Created event: {} - {} - {}",
            namespace, reason, message
        );

        Ok(())
    }

    /// Clean up events older than 1 hour
    async fn cleanup_old_events(&self) -> Result<(), Box<dyn std::error::Error>> {
        let all_events: Vec<Event> = self.storage.list("/registry/events/").await?;
        let one_hour_ago = Utc::now() - chrono::Duration::hours(1);

        for event in all_events {
            if event.last_timestamp < one_hour_ago {
                let namespace = event.metadata.namespace.as_deref().unwrap_or("default");
                let name = &event.metadata.name;
                let key = format!("/registry/events/{}/{}", namespace, name);

                if let Err(e) = self.storage.delete(&key).await {
                    eprintln!("Failed to delete old event {}: {}", name, e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{PodSpec, PodStatus, Container};
    use rusternetes_common::types::ObjectMeta;

    fn create_test_pod(name: &str, namespace: &str, phase: &str, node_name: Option<String>) -> Pod {
        use rusternetes_common::types::{TypeMeta, Phase};

        Pod {
            type_meta: TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name)
                .with_namespace(namespace),
            spec: Some(PodSpec {
                init_containers: None,
                containers: vec![Container {
                    name: "test-container".to_string(),
                    image: "nginx:latest".to_string(),
                    image_pull_policy: None,
                    ports: None,
                    env: None,
                    volume_mounts: None,
                    liveness_probe: None,
                    readiness_probe: None,
                    startup_probe: None,
                    resources: None,
                    working_dir: None,
                    command: None,
                    args: None,
                    security_context: None,
                restart_policy: None,
            }],
                restart_policy: None,
                node_selector: None,
                node_name,
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
                automount_service_account_token: None,
                ephemeral_containers: None,
                overhead: None,
                scheduler_name: None,
                topology_spread_constraints: None,
                resource_claims: None,
            }),
            status: Some(PodStatus {
                phase: Some(match phase {
                    "Pending" => Phase::Pending,
                    "Running" => Phase::Running,
                    "Succeeded" => Phase::Succeeded,
                    "Failed" => Phase::Failed,
                    _ => Phase::Unknown,
                }),
                message: None,
                reason: None,
                pod_ip: None,
                host_ip: None,
                container_statuses: None,
                init_container_statuses: None,
            ephemeral_container_statuses: None,
            }),
        }
    }

    #[test]
    fn test_event_creation() {
        let obj_ref = ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some("default".to_string()),
            name: Some("test-pod".to_string()),
            uid: Some("abc123".to_string()),
            api_version: Some("v1".to_string()),
            resource_version: None,
            field_path: None,
        };

        let event = Event::new(
            "test-event".to_string(),
            "default".to_string(),
            obj_ref,
            "Started".to_string(),
            "Pod started successfully".to_string(),
            EventType::Normal,
        );

        assert_eq!(event.reason, "Started");
        assert_eq!(event.message, "Pod started successfully");
        assert_eq!(event.count, 1);
    }
}

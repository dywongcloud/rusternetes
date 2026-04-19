/// APIService Availability Controller
///
/// Watches APIService resources and updates their Available condition
/// based on whether the backing service has ready endpoints.
///
/// K8s ref: staging/src/k8s.io/kube-aggregator/pkg/controllers/status/remote/remote_available_controller.go
use anyhow::Result;
use rusternetes_common::resources::EndpointSlice;
use rusternetes_storage::{build_prefix, Storage};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

pub struct APIServiceAvailabilityController<S: Storage> {
    storage: Arc<S>,
    interval: Duration,
}

impl<S: Storage + 'static> APIServiceAvailabilityController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(10),
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting APIServiceAvailabilityController");
        loop {
            if let Err(e) = self.reconcile_all().await {
                warn!("APIService availability reconcile error: {}", e);
            }
            tokio::time::sleep(self.interval).await;
        }
    }

    async fn reconcile_all(&self) -> Result<()> {
        let prefix = build_prefix("apiservices", None);
        let apiservices: Vec<serde_json::Value> =
            self.storage.list(&prefix).await.unwrap_or_default();

        for apiservice in apiservices {
            if let Err(e) = self.reconcile_one(&apiservice).await {
                debug!("Failed to reconcile APIService: {}", e);
            }
        }
        Ok(())
    }

    async fn reconcile_one(&self, apiservice: &serde_json::Value) -> Result<()> {
        let name = apiservice
            .pointer("/metadata/name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if name.is_empty() {
            return Ok(());
        }

        // Only process APIServices with a backing service (remote, not local)
        let svc_name = match apiservice
            .pointer("/spec/service/name")
            .and_then(|v| v.as_str())
        {
            Some(n) => n.to_string(),
            None => {
                // Local APIService (no service backing) — always available
                self.update_condition(name, "True", "Local", "Local APIService is always available")
                    .await?;
                return Ok(());
            }
        };
        let svc_ns = apiservice
            .pointer("/spec/service/namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        // Check if the backing service exists
        let svc_key = rusternetes_storage::build_key("services", Some(&svc_ns), &svc_name);
        let svc = match self
            .storage
            .get::<serde_json::Value>(&svc_key)
            .await
        {
            Ok(s) => s,
            Err(_) => {
                self.update_condition(
                    name,
                    "False",
                    "ServiceNotFound",
                    &format!(
                        "service/{} in \"{}\" is not present",
                        svc_name, svc_ns
                    ),
                )
                .await?;
                return Ok(());
            }
        };

        // Get the service port from APIService spec
        let apiservice_port = apiservice
            .pointer("/spec/service/port")
            .and_then(|v| v.as_i64())
            .unwrap_or(443) as i32;

        // Find the matching port name in the service
        let mut port_name = String::new();
        let mut found_port = false;
        if let Some(ports) = svc.pointer("/spec/ports").and_then(|v| v.as_array()) {
            for port in ports {
                let p = port.get("port").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                if p == apiservice_port {
                    found_port = true;
                    port_name = port
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    break;
                }
            }
        }

        if !found_port {
            self.update_condition(
                name,
                "False",
                "ServicePortError",
                &format!(
                    "service/{} in \"{}\" is not listening on port {}",
                    svc_name, svc_ns, apiservice_port
                ),
            )
            .await?;
            return Ok(());
        }

        // Check EndpointSlices for the service
        // K8s ref: the aggregator checks endpointslices for ready endpoints with the matching port
        let ep_prefix = build_prefix("endpointslices", Some(&svc_ns));
        let all_slices: Vec<EndpointSlice> =
            self.storage.list(&ep_prefix).await.unwrap_or_default();

        // Filter slices belonging to this service
        let service_slices: Vec<&EndpointSlice> = all_slices
            .iter()
            .filter(|slice| {
                slice
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("kubernetes.io/service-name"))
                    .map(|sn| sn == &svc_name)
                    .unwrap_or(false)
            })
            .collect();

        if service_slices.is_empty() {
            // No endpoint slices — check Endpoints resource as fallback
            let ep_key = rusternetes_storage::build_key("endpoints", Some(&svc_ns), &svc_name);
            if let Ok(ep) = self
                .storage
                .get::<rusternetes_common::resources::Endpoints>(&ep_key)
                .await
            {
                let has_ready = ep.subsets.iter().any(|s| {
                    s.addresses
                        .as_ref()
                        .map(|addrs| !addrs.is_empty())
                        .unwrap_or(false)
                });
                if has_ready {
                    self.update_condition(
                        name,
                        "True",
                        "Passed",
                        "all checks passed",
                    )
                    .await?;
                    return Ok(());
                }
            }

            self.update_condition(
                name,
                "False",
                "EndpointsNotFound",
                &format!(
                    "cannot find endpointslices for service/{} in \"{}\"",
                    svc_name, svc_ns
                ),
            )
            .await?;
            return Ok(());
        }

        // Check for at least one ready endpoint with the matching port
        let mut has_active = false;
        'outer: for slice in &service_slices {
            let has_ready_endpoint = slice.endpoints.iter().any(|ep| {
                ep.conditions
                    .as_ref()
                    .map(|c| c.ready.unwrap_or(true)) // nil ready = ready
                    .unwrap_or(true)
            });
            if !has_ready_endpoint {
                continue;
            }
            // Check if the slice has the matching port
            for ep_port in &slice.ports {
                let ep_port_name = ep_port.name.as_deref().unwrap_or("");
                if ep_port_name == port_name && ep_port.port.is_some() {
                    has_active = true;
                    break 'outer;
                }
            }
        }

        if has_active {
            self.update_condition(name, "True", "Passed", "all checks passed")
                .await?;
        } else {
            self.update_condition(
                name,
                "False",
                "MissingEndpoints",
                &format!(
                    "endpointslices for service/{} in \"{}\" have no addresses with port name \"{}\"",
                    svc_name, svc_ns, port_name
                ),
            )
            .await?;
        }

        Ok(())
    }

    async fn update_condition(
        &self,
        apiservice_name: &str,
        status: &str,
        reason: &str,
        message: &str,
    ) -> Result<()> {
        let key = rusternetes_storage::build_key("apiservices", None, apiservice_name);
        let mut apiservice: serde_json::Value = match self.storage.get(&key).await {
            Ok(v) => v,
            Err(_) => return Ok(()), // APIService was deleted
        };

        // Check if the condition already matches — skip update if unchanged
        if let Some(conditions) = apiservice
            .pointer("/status/conditions")
            .and_then(|v| v.as_array())
        {
            for c in conditions {
                if c.get("type").and_then(|v| v.as_str()) == Some("Available")
                    && c.get("status").and_then(|v| v.as_str()) == Some(status)
                    && c.get("reason").and_then(|v| v.as_str()) == Some(reason)
                {
                    // Already up to date
                    return Ok(());
                }
            }
        }

        let now = chrono::Utc::now().to_rfc3339();
        let condition = serde_json::json!({
            "type": "Available",
            "status": status,
            "lastTransitionTime": now,
            "reason": reason,
            "message": message,
        });

        // Update the status conditions
        if let Some(status_obj) = apiservice.get_mut("status") {
            if let Some(conditions) = status_obj.get_mut("conditions") {
                if let Some(arr) = conditions.as_array_mut() {
                    // Replace existing Available condition
                    let mut found = false;
                    for c in arr.iter_mut() {
                        if c.get("type").and_then(|v| v.as_str()) == Some("Available") {
                            *c = condition.clone();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        arr.push(condition);
                    }
                }
            } else {
                status_obj["conditions"] = serde_json::json!([condition]);
            }
        } else {
            apiservice["status"] = serde_json::json!({
                "conditions": [condition]
            });
        }

        debug!(
            "Updating APIService {} Available condition: {} ({})",
            apiservice_name, status, reason
        );
        let _ = self.storage.update(&key, &apiservice).await;
        Ok(())
    }
}

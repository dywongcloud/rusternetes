// Admission webhook client for calling external webhooks
//
// This module implements the client for calling external admission webhooks
// and processing their responses.

use rusternetes_common::{
    admission::{
        AdmissionResponse, AdmissionReview, AdmissionReviewRequest, AdmissionReviewResponse,
        GroupVersionKind, GroupVersionResource, Operation, PatchOperation, UserInfo,
    },
    resources::{
        FailurePolicy, MutatingWebhook, MutatingWebhookConfiguration, OperationType, Rule,
        ValidatingWebhook, ValidatingWebhookConfiguration, WebhookClientConfig,
    },
    Result,
};
use rusternetes_storage::Storage;
use serde_json::Value;
use std::error::Error as StdError;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Admission webhook client for calling external webhooks
pub struct AdmissionWebhookClient {
    http_client: reqwest::Client,
}

impl AdmissionWebhookClient {
    /// Create a new admission webhook client
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Call a validating webhook
    pub async fn call_validating_webhook(
        &self,
        webhook: &ValidatingWebhook,
        request: &AdmissionReviewRequest,
    ) -> Result<AdmissionReviewResponse> {
        let url = self.build_webhook_url(&webhook.client_config)?;
        let timeout = webhook
            .timeout_seconds
            .map(|t| Duration::from_secs(t as u64))
            .unwrap_or(Duration::from_secs(10));

        info!("Calling validating webhook {} at {}", webhook.name, url);

        let review = AdmissionReview::new_request(request.clone());

        match self.call_webhook(&url, &review, timeout).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let failure_policy = webhook
                    .failure_policy
                    .as_ref()
                    .unwrap_or(&FailurePolicy::Fail);

                match failure_policy {
                    FailurePolicy::Ignore => {
                        warn!(
                            "Webhook {} failed but FailurePolicy is Ignore: {}",
                            webhook.name, e
                        );
                        // Allow the request despite the error
                        Ok(AdmissionReviewResponse::allow(request.uid.clone()))
                    }
                    FailurePolicy::Fail => {
                        error!(
                            "Webhook {} failed with FailurePolicy Fail: {}",
                            webhook.name, e
                        );
                        Err(e)
                    }
                }
            }
        }
    }

    /// Call a mutating webhook
    pub async fn call_mutating_webhook(
        &self,
        webhook: &MutatingWebhook,
        request: &AdmissionReviewRequest,
    ) -> Result<AdmissionReviewResponse> {
        let url = self.build_webhook_url(&webhook.client_config)?;
        let timeout = webhook
            .timeout_seconds
            .map(|t| Duration::from_secs(t as u64))
            .unwrap_or(Duration::from_secs(10));

        info!("Calling mutating webhook {} at {}", webhook.name, url);

        let review = AdmissionReview::new_request(request.clone());

        match self.call_webhook(&url, &review, timeout).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let failure_policy = webhook
                    .failure_policy
                    .as_ref()
                    .unwrap_or(&FailurePolicy::Fail);

                match failure_policy {
                    FailurePolicy::Ignore => {
                        warn!(
                            "Webhook {} failed but FailurePolicy is Ignore: {}",
                            webhook.name, e
                        );
                        // Allow the request despite the error
                        Ok(AdmissionReviewResponse::allow(request.uid.clone()))
                    }
                    FailurePolicy::Fail => {
                        error!(
                            "Webhook {} failed with FailurePolicy Fail: {}",
                            webhook.name, e
                        );
                        Err(e)
                    }
                }
            }
        }
    }

    /// Internal method to call a webhook
    async fn call_webhook(
        &self,
        url: &str,
        review: &AdmissionReview,
        timeout: Duration,
    ) -> Result<AdmissionReviewResponse> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(5))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| {
                rusternetes_common::Error::Network(format!("Failed to create HTTP client: {}", e))
            })?;

        let response = client.post(url).json(review).send().await.map_err(|e| {
            // Build full error cause chain for diagnostics
            let mut causes = Vec::new();
            let mut source: Option<&dyn StdError> = StdError::source(&e);
            while let Some(cause) = source {
                causes.push(format!("{}", cause));
                source = cause.source();
            }
            let detail = if e.is_connect() {
                "connection refused/failed"
            } else if e.is_timeout() {
                "timeout"
            } else if e.is_request() {
                "request error"
            } else {
                "unknown"
            };
            let cause_chain = if causes.is_empty() {
                String::new()
            } else {
                format!(" causes=[{}]", causes.join(" -> "))
            };
            error!(
                "Webhook call to {} failed: {} ({}){}",
                url, e, detail, cause_chain
            );
            rusternetes_common::Error::Network(format!("Webhook request failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(rusternetes_common::Error::Network(format!(
                "Webhook returned status: {}",
                response.status()
            )));
        }

        let body_bytes = response.bytes().await.map_err(|e| {
            rusternetes_common::Error::Network(format!(
                "Failed to read webhook response body: {}",
                e
            ))
        })?;

        // Try parsing as AdmissionReview first, fall back to parsing as raw Value
        // to extract the response even if there are unknown fields
        let review_response: AdmissionReview = match serde_json::from_slice(&body_bytes) {
            Ok(r) => r,
            Err(e) => {
                // Try parsing as raw JSON and extract the response field
                tracing::warn!(
                    "Webhook response strict parse failed ({}), trying lenient parse. Body: {}",
                    e,
                    String::from_utf8_lossy(&body_bytes[..body_bytes.len().min(500)])
                );
                let value: serde_json::Value = serde_json::from_slice(&body_bytes).map_err(
                    |e2| {
                        rusternetes_common::Error::Network(format!(
                            "Failed to parse webhook response as JSON: {}",
                            e2
                        ))
                    },
                )?;
                // Build AdmissionReview from raw value
                let api_version = value
                    .get("apiVersion")
                    .and_then(|v| v.as_str())
                    .unwrap_or("admission.k8s.io/v1")
                    .to_string();
                let kind = value
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("AdmissionReview")
                    .to_string();
                let response_val = value.get("response");
                let resp = response_val
                    .map(|v| serde_json::from_value::<AdmissionReviewResponse>(v.clone()))
                    .transpose()
                    .map_err(|e| {
                        rusternetes_common::Error::Network(format!(
                            "Failed to parse webhook response.response: {}",
                            e
                        ))
                    })?;
                AdmissionReview {
                    api_version,
                    kind,
                    request: None,
                    response: resp,
                }
            }
        };

        review_response.response.ok_or_else(|| {
            rusternetes_common::Error::Network(
                "Webhook response missing response field".to_string(),
            )
        })
    }

    /// Build webhook URL from client config
    fn build_webhook_url(&self, config: &WebhookClientConfig) -> Result<String> {
        if let Some(ref url) = config.url {
            return Ok(url.clone());
        }

        if let Some(ref service) = config.service {
            // Build service URL — use DNS-style name that will be resolved to endpoint IP
            let namespace = &service.namespace;
            let name = &service.name;
            let path = service.path.as_deref().unwrap_or("/");
            let port = service.port.unwrap_or(443);

            // Store service ref for later resolution to endpoint IP
            let url = format!("https://{}.{}.svc:{}{}", name, namespace, port, path);

            return Ok(url);
        }

        Err(rusternetes_common::Error::InvalidResource(
            "Webhook client config must specify either url or service".to_string(),
        ))
    }

    /// Resolve a K8s service URL to an endpoint IP.
    /// The API server can't resolve .svc DNS names — look up the service's
    /// endpoint IPs from storage instead.
    async fn resolve_service_url<S2: Storage>(url: &str, storage: &Arc<S2>) -> String {
        // Parse service name and namespace from URL like https://name.ns.svc:port/path
        let url_without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        let host_and_rest: Vec<&str> = url_without_scheme.splitn(2, '/').collect();
        let host_port: Vec<&str> = host_and_rest[0].splitn(2, ':').collect();
        let host = host_port[0];
        let port = host_port.get(1).unwrap_or(&"443");
        let path = if host_and_rest.len() > 1 {
            format!("/{}", host_and_rest[1])
        } else {
            "/".to_string()
        };

        // Check if host ends with .svc (K8s service)
        if !host.ends_with(".svc") {
            return url.to_string();
        }

        // Parse name.namespace.svc
        let parts: Vec<&str> = host
            .strip_suffix(".svc")
            .unwrap_or(host)
            .splitn(2, '.')
            .collect();
        if parts.len() != 2 {
            return url.to_string();
        }
        let svc_name = parts[0];
        let svc_namespace = parts[1];

        // Look up endpoint IPs from EndpointSlices
        let es_prefix = format!("/registry/endpointslices/{}/", svc_namespace);
        if let Ok(slices) = storage
            .list::<rusternetes_common::resources::EndpointSlice>(&es_prefix)
            .await
        {
            for slice in &slices {
                let matches = slice
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("kubernetes.io/service-name"))
                    .map(|n| n == svc_name)
                    .unwrap_or(false);
                if !matches {
                    continue;
                }
                // Use the endpoint port from the EndpointSlice if available.
                // The service port (e.g. 443) may differ from the container's targetPort
                // (e.g. 8443 or 8444). The EndpointSlice port is the actual port the
                // pod is listening on.
                let ep_port = slice
                    .ports
                    .first()
                    .and_then(|p| p.port)
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| port.to_string());
                for ep in &slice.endpoints {
                    if ep.conditions.as_ref().and_then(|c| c.ready).unwrap_or(true) {
                        if let Some(addr) = ep.addresses.first() {
                            return format!("https://{}:{}{}", addr, ep_port, path);
                        }
                    }
                }
            }
        }

        // Fall back to ClusterIP
        let svc_key = format!("/registry/services/{}/{}", svc_namespace, svc_name);
        if let Ok(svc) = storage
            .get::<rusternetes_common::resources::Service>(&svc_key)
            .await
        {
            if let Some(cluster_ip) = &svc.spec.cluster_ip {
                if !cluster_ip.is_empty() && cluster_ip != "None" {
                    return format!("https://{}:{}{}", cluster_ip, port, path);
                }
            }
        }

        url.to_string()
    }
}

impl Default for AdmissionWebhookClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Admission webhook manager that maintains webhook configurations and calls them
pub struct AdmissionWebhookManager<S: Storage> {
    storage: Arc<S>,
    client: AdmissionWebhookClient,
}

impl<S: Storage> AdmissionWebhookManager<S> {
    /// Create a new admission webhook manager
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            client: AdmissionWebhookClient::new(),
        }
    }

    /// Run validating webhooks for an admission request
    pub async fn run_validating_webhooks(
        &self,
        operation: &Operation,
        gvk: &GroupVersionKind,
        gvr: &GroupVersionResource,
        namespace: Option<&str>,
        name: &str,
        object: Option<Value>,
        old_object: Option<Value>,
        user_info: &UserInfo,
    ) -> Result<AdmissionResponse> {
        // Load all ValidatingWebhookConfigurations
        let configs: Vec<ValidatingWebhookConfiguration> = self
            .storage
            .list("/registry/validatingwebhookconfigurations/")
            .await?;

        let mut all_warnings = Vec::new();

        for config in configs {
            if let Some(webhooks) = &config.webhooks {
                for webhook in webhooks {
                    // Check if this webhook applies to this request
                    if !self.webhook_matches(&webhook.rules, operation, gvk, gvr, namespace) {
                        continue;
                    }

                    // Skip webhooks whose service no longer exists or namespace is terminating
                    if let Some(ref svc) = webhook.client_config.service {
                        let ns_key =
                            rusternetes_storage::build_key("namespaces", None, &svc.namespace);
                        let ns_gone = match self.storage.get::<serde_json::Value>(&ns_key).await {
                            Err(_) => true,
                            Ok(ns_val) => {
                                // Also skip if namespace is Terminating
                                ns_val.pointer("/status/phase").and_then(|p| p.as_str())
                                    == Some("Terminating")
                                    || ns_val
                                        .get("metadata")
                                        .and_then(|m| m.get("deletionTimestamp"))
                                        .is_some()
                            }
                        };
                        if ns_gone {
                            warn!("Skipping validating webhook {} — service namespace {} no longer exists or is terminating", webhook.name, svc.namespace);
                            continue;
                        }
                    }

                    info!(
                        "Running validating webhook {} for {}/{}",
                        webhook.name, gvk.kind, name
                    );

                    // Build admission request
                    let request = AdmissionReviewRequest {
                        uid: uuid::Uuid::new_v4().to_string(),
                        kind: gvk.clone(),
                        resource: gvr.clone(),
                        sub_resource: None,
                        request_kind: Some(gvk.clone()),
                        request_resource: Some(gvr.clone()),
                        request_sub_resource: None,
                        name: name.to_string(),
                        namespace: namespace.map(|s| s.to_string()),
                        operation: operation.clone(),
                        user_info: user_info.clone(),
                        object: object.clone(),
                        old_object: old_object.clone(),
                        dry_run: None,
                        options: None,
                    };

                    // Call the webhook
                    // Resolve webhook URL — K8s service names need endpoint IP lookup
                    let raw_url = self.client.build_webhook_url(&webhook.client_config)?;
                    let resolved_url =
                        AdmissionWebhookClient::resolve_service_url(&raw_url, &self.storage).await;
                    let timeout = webhook
                        .timeout_seconds
                        .map(|t| Duration::from_secs(t as u64))
                        .unwrap_or(Duration::from_secs(10));
                    let review = AdmissionReview::new_request(request.clone());
                    let response = match self
                        .client
                        .call_webhook(&resolved_url, &review, timeout)
                        .await
                    {
                        Ok(resp) => resp,
                        Err(e) => {
                            let fp = webhook
                                .failure_policy
                                .as_ref()
                                .unwrap_or(&FailurePolicy::Fail);
                            match fp {
                                FailurePolicy::Ignore => {
                                    warn!("Webhook {} failed (Ignore): {}", webhook.name, e);
                                    AdmissionReviewResponse {
                                        uid: request.uid.clone(),
                                        allowed: true,
                                        status: None,
                                        patch: None,
                                        patch_type: None,
                                        audit_annotations: None,
                                        warnings: None,
                                    }
                                }
                                _ => return Err(e),
                            }
                        }
                    };

                    // Collect warnings
                    if let Some(warnings) = &response.warnings {
                        all_warnings.extend(warnings.clone());
                    }

                    // Check if request was denied
                    if !response.allowed {
                        let reason = response
                            .status
                            .as_ref()
                            .and_then(|s| s.message.as_ref())
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("Denied by webhook {}", webhook.name));

                        return Ok(AdmissionResponse::Deny(reason));
                    }
                }
            }
        }

        // All validating webhooks passed
        if !all_warnings.is_empty() {
            info!("Validating webhooks returned warnings: {:?}", all_warnings);
        }

        Ok(AdmissionResponse::Allow)
    }

    /// Run mutating webhooks for an admission request
    pub async fn run_mutating_webhooks(
        &self,
        operation: &Operation,
        gvk: &GroupVersionKind,
        gvr: &GroupVersionResource,
        namespace: Option<&str>,
        name: &str,
        mut object: Option<Value>,
        old_object: Option<Value>,
        user_info: &UserInfo,
    ) -> Result<(AdmissionResponse, Option<Value>)> {
        // Load all MutatingWebhookConfigurations
        let configs: Vec<MutatingWebhookConfiguration> = self
            .storage
            .list("/registry/mutatingwebhookconfigurations/")
            .await?;

        let mut all_patches = Vec::new();
        let mut all_warnings = Vec::new();

        for config in configs {
            if let Some(webhooks) = &config.webhooks {
                for webhook in webhooks {
                    // Check if this webhook applies to this request
                    if !self.webhook_matches(&webhook.rules, operation, gvk, gvr, namespace) {
                        continue;
                    }

                    // Skip webhooks whose service no longer exists or namespace is terminating
                    if let Some(ref svc) = webhook.client_config.service {
                        let ns_key =
                            rusternetes_storage::build_key("namespaces", None, &svc.namespace);
                        let ns_gone = match self.storage.get::<serde_json::Value>(&ns_key).await {
                            Err(_) => true,
                            Ok(ns_val) => {
                                ns_val.pointer("/status/phase").and_then(|p| p.as_str())
                                    == Some("Terminating")
                                    || ns_val
                                        .get("metadata")
                                        .and_then(|m| m.get("deletionTimestamp"))
                                        .is_some()
                            }
                        };
                        if ns_gone {
                            warn!(
                                "Skipping webhook {} — service namespace {} no longer exists",
                                webhook.name, svc.namespace
                            );
                            continue;
                        }
                    }

                    info!(
                        "Running mutating webhook {} for {}/{}",
                        webhook.name, gvk.kind, name
                    );

                    // Build admission request with potentially mutated object
                    let request = AdmissionReviewRequest {
                        uid: uuid::Uuid::new_v4().to_string(),
                        kind: gvk.clone(),
                        resource: gvr.clone(),
                        sub_resource: None,
                        request_kind: Some(gvk.clone()),
                        request_resource: Some(gvr.clone()),
                        request_sub_resource: None,
                        name: name.to_string(),
                        namespace: namespace.map(|s| s.to_string()),
                        operation: operation.clone(),
                        user_info: user_info.clone(),
                        object: object.clone(),
                        old_object: old_object.clone(),
                        dry_run: None,
                        options: None,
                    };

                    // Resolve webhook URL — K8s service names need endpoint IP lookup
                    let raw_url = self.client.build_webhook_url(&webhook.client_config)?;
                    let resolved_url =
                        AdmissionWebhookClient::resolve_service_url(&raw_url, &self.storage).await;
                    let timeout = webhook
                        .timeout_seconds
                        .map(|t| Duration::from_secs(t as u64))
                        .unwrap_or(Duration::from_secs(10));
                    let review = AdmissionReview::new_request(request.clone());
                    let response = match self
                        .client
                        .call_webhook(&resolved_url, &review, timeout)
                        .await
                    {
                        Ok(resp) => resp,
                        Err(e) => {
                            let fp = webhook
                                .failure_policy
                                .as_ref()
                                .unwrap_or(&FailurePolicy::Fail);
                            match fp {
                                FailurePolicy::Ignore => {
                                    warn!(
                                        "Mutating webhook {} failed (Ignore): {}",
                                        webhook.name, e
                                    );
                                    AdmissionReviewResponse {
                                        uid: request.uid.clone(),
                                        allowed: true,
                                        status: None,
                                        patch: None,
                                        patch_type: None,
                                        audit_annotations: None,
                                        warnings: None,
                                    }
                                }
                                _ => return Err(e),
                            }
                        }
                    };

                    // Collect warnings
                    if let Some(warnings) = &response.warnings {
                        all_warnings.extend(warnings.clone());
                    }

                    // Check if request was denied
                    if !response.allowed {
                        let reason = response
                            .status
                            .as_ref()
                            .and_then(|s| s.message.as_ref())
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("Denied by webhook {}", webhook.name));

                        return Ok((AdmissionResponse::Deny(reason), object));
                    }

                    // Apply patches
                    if let Some(patch_base64) = &response.patch {
                        // Decode base64 patch
                        use base64::Engine;
                        let patch_bytes = base64::engine::general_purpose::STANDARD
                            .decode(patch_base64)
                            .map_err(|e| {
                                rusternetes_common::Error::InvalidResource(format!(
                                    "Failed to decode webhook patch: {}",
                                    e
                                ))
                            })?;

                        let patch_str = String::from_utf8(patch_bytes).map_err(|e| {
                            rusternetes_common::Error::InvalidResource(format!(
                                "Failed to parse webhook patch as UTF-8: {}",
                                e
                            ))
                        })?;

                        let patches: Vec<PatchOperation> = serde_json::from_str(&patch_str)
                            .map_err(|e| {
                                rusternetes_common::Error::InvalidResource(format!(
                                    "Failed to parse webhook patch as JSON: {}",
                                    e
                                ))
                            })?;

                        // Apply patches to object
                        if let Some(ref mut obj) = object {
                            for patch in &patches {
                                apply_json_patch(obj, patch)?;
                            }
                        }

                        all_patches.extend(patches);
                    }
                }
            }
        }

        // All mutating webhooks passed
        if !all_warnings.is_empty() {
            info!("Mutating webhooks returned warnings: {:?}", all_warnings);
        }

        let response = if all_patches.is_empty() {
            AdmissionResponse::Allow
        } else {
            AdmissionResponse::AllowWithPatch(all_patches)
        };

        Ok((response, object))
    }

    /// Check if a webhook matches the given request
    fn webhook_matches(
        &self,
        rules: &[rusternetes_common::resources::RuleWithOperations],
        operation: &Operation,
        gvk: &GroupVersionKind,
        gvr: &GroupVersionResource,
        namespace: Option<&str>,
    ) -> bool {
        for rule in rules {
            // Check if operation matches
            if !self.operation_matches(&rule.operations, operation) {
                continue;
            }

            // Check if resource matches
            if !self.resource_matches(&rule.rule, gvk, gvr) {
                continue;
            }

            // Check if scope matches
            if let Some(scope) = &rule.rule.scope {
                if scope == "Namespaced" && namespace.is_none() {
                    continue;
                }
                if scope == "Cluster" && namespace.is_some() {
                    continue;
                }
            }

            // Rule matches!
            return true;
        }

        false
    }

    /// Check if operation matches webhook rule
    fn operation_matches(&self, operations: &[OperationType], operation: &Operation) -> bool {
        for op in operations {
            match op {
                OperationType::All => return true,
                OperationType::Create if matches!(operation, Operation::Create) => return true,
                OperationType::Update if matches!(operation, Operation::Update) => return true,
                OperationType::Delete if matches!(operation, Operation::Delete) => return true,
                OperationType::Connect if matches!(operation, Operation::Connect) => return true,
                _ => continue,
            }
        }
        false
    }

    /// Check if resource matches webhook rule
    fn resource_matches(
        &self,
        rule: &Rule,
        _gvk: &GroupVersionKind,
        gvr: &GroupVersionResource,
    ) -> bool {
        // Check API group
        if !rule.api_groups.contains(&"*".to_string()) && !rule.api_groups.contains(&gvr.group) {
            return false;
        }

        // Check API version
        if !rule.api_versions.contains(&"*".to_string())
            && !rule.api_versions.contains(&gvr.version)
        {
            return false;
        }

        // Check resource
        if !rule.resources.contains(&"*".to_string()) && !rule.resources.contains(&gvr.resource) {
            return false;
        }

        true
    }
}

/// Apply a single JSON patch operation to an object
fn apply_json_patch(object: &mut Value, patch: &PatchOperation) -> Result<()> {
    use rusternetes_common::admission::PatchOp;

    match patch.op {
        PatchOp::Add => {
            if let Some(value) = &patch.value {
                apply_json_pointer_add(object, &patch.path, value.clone())?;
            }
        }
        PatchOp::Remove => {
            apply_json_pointer_remove(object, &patch.path)?;
        }
        PatchOp::Replace => {
            if let Some(value) = &patch.value {
                apply_json_pointer_replace(object, &patch.path, value.clone())?;
            }
        }
        _ => {
            // For now, only support add, remove, replace
            warn!("Unsupported patch operation: {:?}", patch.op);
        }
    }

    Ok(())
}

/// Apply JSON pointer add operation
fn apply_json_pointer_add(object: &mut Value, path: &str, value: Value) -> Result<()> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.is_empty() || parts[0].is_empty() {
        *object = value;
        return Ok(());
    }

    let mut current = object;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - add the value
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), value.clone());
            }
        } else {
            // Navigate to the next level
            current = current
                .as_object_mut()
                .and_then(|obj| obj.get_mut(*part))
                .ok_or_else(|| {
                    rusternetes_common::Error::InvalidResource(format!("Path not found: {}", path))
                })?;
        }
    }

    Ok(())
}

/// Apply JSON pointer remove operation
fn apply_json_pointer_remove(object: &mut Value, path: &str) -> Result<()> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.is_empty() || parts[0].is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "Cannot remove root".to_string(),
        ));
    }

    let mut current = object;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - remove the value
            if let Some(obj) = current.as_object_mut() {
                obj.remove(*part);
            }
        } else {
            // Navigate to the next level
            current = current
                .as_object_mut()
                .and_then(|obj| obj.get_mut(*part))
                .ok_or_else(|| {
                    rusternetes_common::Error::InvalidResource(format!("Path not found: {}", path))
                })?;
        }
    }

    Ok(())
}

impl<S: Storage> AdmissionWebhookManager<S> {
    /// Run ValidatingAdmissionPolicy checks for an admission request.
    /// Evaluates CEL expressions from matching policies and rejects if any Deny action matches.
    ///
    /// `resource` is the plural resource name (e.g. "configmaps", "pods", "deployments").
    /// If provided, it is used for more accurate resource rule matching.
    /// `namespace` is the namespace of the object (for namespaced resources).
    /// `old_object` is the previous version (for UPDATE operations).
    pub async fn run_validating_admission_policies(
        &self,
        operation: &Operation,
        gvk: &GroupVersionKind,
        object: Option<&Value>,
    ) -> Result<()> {
        self.run_validating_admission_policies_ext(operation, gvk, object, None, None, None)
            .await
    }

    /// Extended VAP evaluation with resource name and namespace for precise matching.
    pub async fn run_validating_admission_policies_ext(
        &self,
        operation: &Operation,
        gvk: &GroupVersionKind,
        object: Option<&Value>,
        old_object: Option<&Value>,
        resource: Option<&str>,
        namespace: Option<&str>,
    ) -> Result<()> {
        use rusternetes_common::CELEvaluator;

        // Load all ValidatingAdmissionPolicies
        let policies: Vec<Value> = self
            .storage
            .list("/registry/validatingadmissionpolicies/")
            .await
            .unwrap_or_default();

        if policies.is_empty() {
            return Ok(());
        }

        // Load all ValidatingAdmissionPolicyBindings
        let bindings: Vec<Value> = self
            .storage
            .list("/registry/validatingadmissionpolicybindings/")
            .await
            .unwrap_or_default();

        let mut evaluator = CELEvaluator::new();

        // Derive resource name from kind if not provided
        let derived_resource = resource
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}s", gvk.kind.to_lowercase()));

        let op_str = match operation {
            Operation::Create => "CREATE",
            Operation::Update => "UPDATE",
            Operation::Delete => "DELETE",
            _ => "",
        };

        for policy in &policies {
            let policy_name = policy
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");

            // Find the binding that references this policy
            let matching_binding = bindings.iter().find(|b| {
                b.get("spec")
                    .and_then(|s| s.get("policyName"))
                    .and_then(|n| n.as_str())
                    == Some(policy_name)
            });
            if matching_binding.is_none() {
                continue;
            }
            let binding = matching_binding.unwrap();

            // Check match conditions from spec.matchConstraints
            let match_resources = policy
                .get("spec")
                .and_then(|s| s.get("matchConstraints"))
                .and_then(|m| m.get("resourceRules"));
            if let Some(rules) = match_resources {
                if let Some(rules_arr) = rules.as_array() {
                    let matches = rules_arr.iter().any(|rule| {
                        let api_groups = rule.get("apiGroups").and_then(|g| g.as_array());
                        let resources = rule.get("resources").and_then(|r| r.as_array());
                        let ops = rule.get("operations").and_then(|o| o.as_array());

                        let group_match = api_groups.map_or(true, |groups| {
                            groups.iter().any(|g| {
                                let gs = g.as_str().unwrap_or("");
                                gs == "*" || gs == gvk.group
                            })
                        });
                        let resource_match = resources.map_or(true, |res| {
                            res.iter().any(|r| {
                                let rs = r.as_str().unwrap_or("");
                                rs == "*" || rs == derived_resource
                            })
                        });
                        let op_match = ops.map_or(true, |operations| {
                            operations.iter().any(|o| {
                                let os = o.as_str().unwrap_or("");
                                os == "*" || os == op_str
                            })
                        });
                        group_match && resource_match && op_match
                    });
                    if !matches {
                        continue;
                    }
                }
            }

            // Check matchConditions from the policy spec
            let match_conditions_pass = self.evaluate_match_conditions(
                policy,
                object,
                old_object,
                operation,
                gvk,
                namespace,
                &mut evaluator,
            );
            if !match_conditions_pass {
                continue;
            }

            // Build CEL context with object variable
            let mut context = rusternetes_common::CELContext::new();
            if let Some(obj) = object {
                let _ = context.add_json_variable("object", obj);
            }

            // Add oldObject for UPDATE operations
            if let Some(old) = old_object {
                let _ = context.add_json_variable("oldObject", old);
            } else {
                // For non-update ops, oldObject is null
                let _ = context.add_json_variable("oldObject", &serde_json::Value::Null);
            }

            // Add request context (K8s conformance tests access request.operation, etc.)
            let request_val = serde_json::json!({
                "operation": op_str,
                "kind": {
                    "group": gvk.group,
                    "version": gvk.version,
                    "kind": gvk.kind,
                },
                "resource": {
                    "group": gvk.group,
                    "version": gvk.version,
                    "resource": derived_resource,
                },
                "namespace": namespace.unwrap_or(""),
                "name": object.and_then(|o| o.get("metadata")).and_then(|m| m.get("name")).and_then(|n| n.as_str()).unwrap_or(""),
                "userInfo": {
                    "username": "system:admin",
                    "groups": ["system:masters", "system:authenticated"],
                },
            });
            let _ = context.add_json_variable("request", &request_val);

            // Add params from the binding's paramRef (if present)
            if let Some(param_ref) = binding.get("spec").and_then(|s| s.get("paramRef")) {
                let param_ns = param_ref
                    .get("namespace")
                    .and_then(|n| n.as_str())
                    .or(namespace);
                let param_name = param_ref.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let param_kind = param_ref.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                let param_api_group = param_ref
                    .get("apiGroup")
                    .and_then(|g| g.as_str())
                    .unwrap_or("");

                if !param_name.is_empty() {
                    // Try to load the param resource from storage
                    let resource_type = format!("{}s", param_kind.to_lowercase());
                    let param_key = if let Some(ns) = param_ns {
                        format!("/registry/{}/{}/{}", resource_type, ns, param_name)
                    } else {
                        format!("/registry/{}/{}", resource_type, param_name)
                    };
                    if let Ok(param_val) = self.storage.get::<serde_json::Value>(&param_key).await {
                        let _ = context.add_json_variable("params", &param_val);
                    } else {
                        // Try as CRD instance
                        let crd_key = format!(
                            "/registry/{}.{}/{}/{}",
                            resource_type,
                            param_api_group,
                            param_ns.unwrap_or(""),
                            param_name
                        );
                        if let Ok(param_val) = self.storage.get::<serde_json::Value>(&crd_key).await
                        {
                            let _ = context.add_json_variable("params", &param_val);
                        } else {
                            let _ = context.add_json_variable("params", &serde_json::Value::Null);
                        }
                    }
                } else {
                    let _ = context.add_json_variable("params", &serde_json::Value::Null);
                }
            } else {
                let _ = context.add_json_variable("params", &serde_json::Value::Null);
            }

            // Add namespaceObject — the Namespace object for the request's namespace.
            // K8s conformance tests use expressions like `namespaceObject.metadata.name`.
            if let Some(ns) = namespace {
                if !ns.is_empty() {
                    let ns_key = format!("/registry/namespaces/{}", ns);
                    if let Ok(ns_val) = self.storage.get::<serde_json::Value>(&ns_key).await {
                        let _ = context.add_json_variable(
                            "namespaceObject",
                            &serde_json::to_value(&ns_val).unwrap_or(serde_json::Value::Null),
                        );
                    } else {
                        // If namespace not found in storage, provide a minimal object
                        // so that expressions like namespaceObject.metadata.name don't error.
                        let minimal_ns = serde_json::json!({
                            "apiVersion": "v1",
                            "kind": "Namespace",
                            "metadata": {
                                "name": ns,
                            }
                        });
                        let _ = context.add_json_variable("namespaceObject", &minimal_ns);
                    }
                }
            }

            // Evaluate spec.variables, building a "variables" Map for CEL access.
            // CEL expressions reference variables as `variables.NAME`, which means
            // "variables" must be a Map variable in the CEL context.
            if let Some(vars) = policy
                .get("spec")
                .and_then(|s| s.get("variables"))
                .and_then(|v| v.as_array())
            {
                let mut var_map: std::collections::HashMap<
                    cel_interpreter::objects::Key,
                    cel_interpreter::Value,
                > = std::collections::HashMap::new();
                for var_def in vars {
                    let var_name = var_def.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let var_expr = var_def
                        .get("expression")
                        .and_then(|e| e.as_str())
                        .unwrap_or("");
                    if var_name.is_empty() || var_expr.is_empty() {
                        continue;
                    }
                    // Evaluate the variable expression and add to the variables map
                    match evaluator.evaluate_to_value(var_expr, &context) {
                        Ok(val) => {
                            var_map.insert(
                                cel_interpreter::objects::Key::String(std::sync::Arc::new(
                                    var_name.to_string(),
                                )),
                                val,
                            );
                            // Re-add the updated variables map to context after each variable
                            // so later variables can reference earlier ones
                            context.add_variable(
                                "variables".to_string(),
                                cel_interpreter::Value::Map(cel_interpreter::objects::Map {
                                    map: std::sync::Arc::new(var_map.clone()),
                                }),
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "CEL variable {} evaluation error for policy {}: {}",
                                var_name,
                                policy_name,
                                e
                            );
                        }
                    }
                }
            }

            // Check failure policy
            let failure_policy = policy
                .get("spec")
                .and_then(|s| s.get("failurePolicy"))
                .and_then(|f| f.as_str())
                .unwrap_or("Fail");

            // Evaluate validations
            if let Some(validations) = policy
                .get("spec")
                .and_then(|s| s.get("validations"))
                .and_then(|v| v.as_array())
            {
                for validation in validations {
                    let expression = validation
                        .get("expression")
                        .and_then(|e| e.as_str())
                        .unwrap_or("");
                    if expression.is_empty() {
                        continue;
                    }

                    // Evaluate
                    match evaluator.evaluate(expression, &context) {
                        Ok(true) => {
                            tracing::debug!(
                                "VAP {} expression '{}' passed",
                                policy_name,
                                expression
                            );
                        }
                        Ok(false) => {
                            tracing::info!(
                                "VAP {} expression '{}' DENIED for {} in ns {:?}",
                                policy_name,
                                expression,
                                derived_resource,
                                namespace
                            );
                            // Check validation actions: first from the binding, then from
                            // the validation rule itself, defaulting to Deny if neither set.
                            let actions = binding
                                .get("spec")
                                .and_then(|s| s.get("validationActions"))
                                .and_then(|a| a.as_array())
                                .or_else(|| {
                                    validation
                                        .get("validationActions")
                                        .and_then(|a| a.as_array())
                                });
                            let has_deny = actions.map_or(true, |acts| {
                                acts.iter().any(|a| a.as_str() == Some("Deny"))
                            });
                            if has_deny {
                                // Use messageExpression (CEL) if present, otherwise static message
                                let message = if let Some(msg_expr) =
                                    validation.get("messageExpression").and_then(|m| m.as_str())
                                {
                                    match evaluator.evaluate_to_value(msg_expr, &context) {
                                        Ok(cel_interpreter::Value::String(s)) => s.to_string(),
                                        Ok(other) => format!("{:?}", other),
                                        Err(_) => validation
                                            .get("message")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("Validation failed")
                                            .to_string(),
                                    }
                                } else {
                                    validation
                                        .get("message")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("Validation failed")
                                        .to_string()
                                };
                                return Err(rusternetes_common::Error::InvalidResource(format!(
                                    "ValidatingAdmissionPolicy {} denied: {}",
                                    policy_name, message
                                )));
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "CEL evaluation error for policy {} expression '{}': {}",
                                policy_name,
                                expression,
                                e
                            );
                            // On error, check failure policy
                            if failure_policy == "Fail" {
                                return Err(rusternetes_common::Error::InvalidResource(format!(
                                    "ValidatingAdmissionPolicy {} evaluation error: {}",
                                    policy_name, e
                                )));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Evaluate matchConditions for a policy. Returns true if all conditions pass (or none exist).
    fn evaluate_match_conditions(
        &self,
        policy: &Value,
        object: Option<&Value>,
        old_object: Option<&Value>,
        operation: &Operation,
        gvk: &GroupVersionKind,
        namespace: Option<&str>,
        evaluator: &mut rusternetes_common::CELEvaluator,
    ) -> bool {
        let conditions = match policy
            .get("spec")
            .and_then(|s| s.get("matchConditions"))
            .and_then(|c| c.as_array())
        {
            Some(c) if !c.is_empty() => c,
            _ => return true, // No conditions = always match
        };

        let op_str = match operation {
            Operation::Create => "CREATE",
            Operation::Update => "UPDATE",
            Operation::Delete => "DELETE",
            _ => "",
        };

        let mut context = rusternetes_common::CELContext::new();
        if let Some(obj) = object {
            let _ = context.add_json_variable("object", obj);
        }
        if let Some(old) = old_object {
            let _ = context.add_json_variable("oldObject", old);
        } else {
            let _ = context.add_json_variable("oldObject", &serde_json::Value::Null);
        }
        let request_val = serde_json::json!({
            "operation": op_str,
            "kind": {
                "group": gvk.group,
                "version": gvk.version,
                "kind": gvk.kind,
            },
            "namespace": namespace.unwrap_or(""),
        });
        let _ = context.add_json_variable("request", &request_val);

        for cond in conditions {
            let expr = cond
                .get("expression")
                .and_then(|e| e.as_str())
                .unwrap_or("");
            if expr.is_empty() {
                continue;
            }
            match evaluator.evaluate(expr, &context) {
                Ok(true) => { /* condition matched, continue */ }
                Ok(false) => return false, // condition not met, skip this policy
                Err(_) => return false,    // error evaluating = skip
            }
        }
        true
    }
}

/// Apply JSON pointer replace operation
fn apply_json_pointer_replace(object: &mut Value, path: &str, value: Value) -> Result<()> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.is_empty() || parts[0].is_empty() {
        *object = value;
        return Ok(());
    }

    let mut current = object;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - replace the value
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), value.clone());
            }
        } else {
            // Navigate to the next level
            current = current
                .as_object_mut()
                .and_then(|obj| obj.get_mut(*part))
                .ok_or_else(|| {
                    rusternetes_common::Error::InvalidResource(format!("Path not found: {}", path))
                })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::RuleWithOperations;
    use rusternetes_storage::memory::MemoryStorage;
    use serde_json::json;

    // ===== JSON Patch Tests =====

    #[test]
    fn test_apply_json_patch_add() {
        let mut obj = json!({
            "metadata": {
                "name": "test"
            }
        });

        let patch = PatchOperation {
            op: rusternetes_common::admission::PatchOp::Add,
            path: "/metadata/labels".to_string(),
            value: Some(json!({"app": "test"})),
            from: None,
        };

        apply_json_patch(&mut obj, &patch).unwrap();

        assert_eq!(obj["metadata"]["labels"], json!({"app": "test"}));
    }

    #[test]
    fn test_apply_json_patch_remove() {
        let mut obj = json!({
            "metadata": {
                "name": "test",
                "labels": {"app": "test"}
            }
        });

        let patch = PatchOperation {
            op: rusternetes_common::admission::PatchOp::Remove,
            path: "/metadata/labels".to_string(),
            value: None,
            from: None,
        };

        apply_json_patch(&mut obj, &patch).unwrap();

        assert!(obj["metadata"]["labels"].is_null());
    }

    #[test]
    fn test_apply_json_patch_replace() {
        let mut obj = json!({
            "metadata": {
                "name": "test"
            }
        });

        let patch = PatchOperation {
            op: rusternetes_common::admission::PatchOp::Replace,
            path: "/metadata/name".to_string(),
            value: Some(json!("new-name")),
            from: None,
        };

        apply_json_patch(&mut obj, &patch).unwrap();

        assert_eq!(obj["metadata"]["name"], json!("new-name"));
    }

    #[test]
    fn test_apply_json_patch_nested_add() {
        let mut obj = json!({
            "metadata": {
                "name": "test",
                "annotations": {}
            }
        });

        let patch = PatchOperation {
            op: rusternetes_common::admission::PatchOp::Add,
            path: "/metadata/annotations/key".to_string(),
            value: Some(json!("value")),
            from: None,
        };

        apply_json_patch(&mut obj, &patch).unwrap();

        assert_eq!(obj["metadata"]["annotations"]["key"], json!("value"));
    }

    #[test]
    fn test_apply_json_patch_replace_root() {
        let mut obj = json!({
            "metadata": {
                "name": "test"
            }
        });

        let new_obj = json!({
            "metadata": {
                "name": "replaced"
            }
        });

        let patch = PatchOperation {
            op: rusternetes_common::admission::PatchOp::Replace,
            path: "/".to_string(),
            value: Some(new_obj.clone()),
            from: None,
        };

        apply_json_patch(&mut obj, &patch).unwrap();

        assert_eq!(obj, new_obj);
    }

    #[test]
    fn test_apply_json_patch_remove_error_on_root() {
        let mut obj = json!({
            "metadata": {
                "name": "test"
            }
        });

        let patch = PatchOperation {
            op: rusternetes_common::admission::PatchOp::Remove,
            path: "/".to_string(),
            value: None,
            from: None,
        };

        let result = apply_json_patch(&mut obj, &patch);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot remove root"));
    }

    // ===== Webhook Matching Tests =====

    fn create_test_manager() -> AdmissionWebhookManager<MemoryStorage> {
        let storage = Arc::new(MemoryStorage::new());
        AdmissionWebhookManager::new(storage)
    }

    #[test]
    fn test_operation_matches_create() {
        let manager = create_test_manager();
        let operations = vec![OperationType::Create];

        assert!(manager.operation_matches(&operations, &Operation::Create));
        assert!(!manager.operation_matches(&operations, &Operation::Update));
        assert!(!manager.operation_matches(&operations, &Operation::Delete));
    }

    #[test]
    fn test_operation_matches_all() {
        let manager = create_test_manager();
        let operations = vec![OperationType::All];

        assert!(manager.operation_matches(&operations, &Operation::Create));
        assert!(manager.operation_matches(&operations, &Operation::Update));
        assert!(manager.operation_matches(&operations, &Operation::Delete));
        assert!(manager.operation_matches(&operations, &Operation::Connect));
    }

    #[test]
    fn test_operation_matches_multiple() {
        let manager = create_test_manager();
        let operations = vec![OperationType::Create, OperationType::Update];

        assert!(manager.operation_matches(&operations, &Operation::Create));
        assert!(manager.operation_matches(&operations, &Operation::Update));
        assert!(!manager.operation_matches(&operations, &Operation::Delete));
    }

    #[test]
    fn test_resource_matches_exact() {
        let manager = create_test_manager();
        let rule = Rule {
            api_groups: vec!["".to_string()],
            api_versions: vec!["v1".to_string()],
            resources: vec!["pods".to_string()],
            scope: None,
        };

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "Pod".to_string(),
        };

        let gvr = GroupVersionResource {
            group: "".to_string(),
            version: "v1".to_string(),
            resource: "pods".to_string(),
        };

        assert!(manager.resource_matches(&rule, &gvk, &gvr));
    }

    #[test]
    fn test_resource_matches_wildcard_group() {
        let manager = create_test_manager();
        let rule = Rule {
            api_groups: vec!["*".to_string()],
            api_versions: vec!["v1".to_string()],
            resources: vec!["pods".to_string()],
            scope: None,
        };

        let gvr = GroupVersionResource {
            group: "apps".to_string(),
            version: "v1".to_string(),
            resource: "pods".to_string(),
        };

        let gvk = GroupVersionKind {
            group: "apps".to_string(),
            version: "v1".to_string(),
            kind: "Pod".to_string(),
        };

        assert!(manager.resource_matches(&rule, &gvk, &gvr));
    }

    #[test]
    fn test_resource_matches_wildcard_all() {
        let manager = create_test_manager();
        let rule = Rule {
            api_groups: vec!["*".to_string()],
            api_versions: vec!["*".to_string()],
            resources: vec!["*".to_string()],
            scope: None,
        };

        let gvr = GroupVersionResource {
            group: "apps".to_string(),
            version: "v1".to_string(),
            resource: "deployments".to_string(),
        };

        let gvk = GroupVersionKind {
            group: "apps".to_string(),
            version: "v1".to_string(),
            kind: "Deployment".to_string(),
        };

        assert!(manager.resource_matches(&rule, &gvk, &gvr));
    }

    #[test]
    fn test_resource_matches_mismatch() {
        let manager = create_test_manager();
        let rule = Rule {
            api_groups: vec!["".to_string()],
            api_versions: vec!["v1".to_string()],
            resources: vec!["pods".to_string()],
            scope: None,
        };

        let gvr = GroupVersionResource {
            group: "apps".to_string(),
            version: "v1".to_string(),
            resource: "deployments".to_string(),
        };

        let gvk = GroupVersionKind {
            group: "apps".to_string(),
            version: "v1".to_string(),
            kind: "Deployment".to_string(),
        };

        assert!(!manager.resource_matches(&rule, &gvk, &gvr));
    }

    #[test]
    fn test_webhook_matches_full() {
        let manager = create_test_manager();

        let rules = vec![RuleWithOperations {
            operations: vec![OperationType::Create],
            rule: Rule {
                api_groups: vec!["".to_string()],
                api_versions: vec!["v1".to_string()],
                resources: vec!["pods".to_string()],
                scope: Some("Namespaced".to_string()),
            },
        }];

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "Pod".to_string(),
        };

        let gvr = GroupVersionResource {
            group: "".to_string(),
            version: "v1".to_string(),
            resource: "pods".to_string(),
        };

        assert!(manager.webhook_matches(&rules, &Operation::Create, &gvk, &gvr, Some("default")));
    }

    #[test]
    fn test_webhook_matches_scope_cluster() {
        let manager = create_test_manager();

        let rules = vec![RuleWithOperations {
            operations: vec![OperationType::Create],
            rule: Rule {
                api_groups: vec!["".to_string()],
                api_versions: vec!["v1".to_string()],
                resources: vec!["nodes".to_string()],
                scope: Some("Cluster".to_string()),
            },
        }];

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "Node".to_string(),
        };

        let gvr = GroupVersionResource {
            group: "".to_string(),
            version: "v1".to_string(),
            resource: "nodes".to_string(),
        };

        // Should match for cluster-scoped (no namespace)
        assert!(manager.webhook_matches(&rules, &Operation::Create, &gvk, &gvr, None));

        // Should NOT match for namespaced resources
        assert!(!manager.webhook_matches(&rules, &Operation::Create, &gvk, &gvr, Some("default")));
    }

    #[test]
    fn test_webhook_matches_operation_mismatch() {
        let manager = create_test_manager();

        let rules = vec![RuleWithOperations {
            operations: vec![OperationType::Create],
            rule: Rule {
                api_groups: vec!["".to_string()],
                api_versions: vec!["v1".to_string()],
                resources: vec!["pods".to_string()],
                scope: None,
            },
        }];

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "Pod".to_string(),
        };

        let gvr = GroupVersionResource {
            group: "".to_string(),
            version: "v1".to_string(),
            resource: "pods".to_string(),
        };

        // Should NOT match UPDATE operation
        assert!(!manager.webhook_matches(&rules, &Operation::Update, &gvk, &gvr, Some("default")));
    }

    #[test]
    fn test_webhook_matches_multiple_rules() {
        let manager = create_test_manager();

        let rules = vec![
            RuleWithOperations {
                operations: vec![OperationType::Create],
                rule: Rule {
                    api_groups: vec!["apps".to_string()],
                    api_versions: vec!["v1".to_string()],
                    resources: vec!["deployments".to_string()],
                    scope: None,
                },
            },
            RuleWithOperations {
                operations: vec![OperationType::Create],
                rule: Rule {
                    api_groups: vec!["".to_string()],
                    api_versions: vec!["v1".to_string()],
                    resources: vec!["pods".to_string()],
                    scope: None,
                },
            },
        ];

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "Pod".to_string(),
        };

        let gvr = GroupVersionResource {
            group: "".to_string(),
            version: "v1".to_string(),
            resource: "pods".to_string(),
        };

        // Should match the second rule
        assert!(manager.webhook_matches(&rules, &Operation::Create, &gvk, &gvr, Some("default")));
    }

    // ===== Webhook Client Tests =====

    #[test]
    fn test_build_webhook_url_direct() {
        let client = AdmissionWebhookClient::new();
        let config = WebhookClientConfig {
            url: Some("https://example.com/webhook".to_string()),
            service: None,
            ca_bundle: None,
        };

        let url = client.build_webhook_url(&config).unwrap();
        assert_eq!(url, "https://example.com/webhook");
    }

    #[test]
    fn test_build_webhook_url_service() {
        let client = AdmissionWebhookClient::new();
        let config = WebhookClientConfig {
            url: None,
            service: Some(rusternetes_common::resources::ServiceReference {
                namespace: "webhook-system".to_string(),
                name: "webhook-service".to_string(),
                path: Some("/validate".to_string()),
                port: Some(8443),
            }),
            ca_bundle: None,
        };

        let url = client.build_webhook_url(&config).unwrap();
        assert_eq!(
            url,
            "https://webhook-service.webhook-system.svc:8443/validate"
        );
    }

    #[test]
    fn test_build_webhook_url_service_defaults() {
        let client = AdmissionWebhookClient::new();
        let config = WebhookClientConfig {
            url: None,
            service: Some(rusternetes_common::resources::ServiceReference {
                namespace: "default".to_string(),
                name: "my-webhook".to_string(),
                path: None,
                port: None,
            }),
            ca_bundle: None,
        };

        let url = client.build_webhook_url(&config).unwrap();
        assert_eq!(url, "https://my-webhook.default.svc:443/");
    }

    #[test]
    fn test_build_webhook_url_missing_config() {
        let client = AdmissionWebhookClient::new();
        let config = WebhookClientConfig {
            url: None,
            service: None,
            ca_bundle: None,
        };

        let result = client.build_webhook_url(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must specify either url or service"));
    }

    // ===== ValidatingAdmissionPolicy Tests =====

    #[tokio::test]
    async fn test_vap_denies_configmap_creation() {
        let storage = Arc::new(MemoryStorage::new());
        let manager = AdmissionWebhookManager::new(storage.clone());

        // Create a VAP that denies configmaps with name starting with "deny-"
        let policy = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicy",
            "metadata": {
                "name": "deny-configmaps",
                "creationTimestamp": chrono::Utc::now().to_rfc3339(),
            },
            "spec": {
                "matchConstraints": {
                    "resourceRules": [{
                        "apiGroups": [""],
                        "apiVersions": ["v1"],
                        "resources": ["configmaps"],
                        "operations": ["CREATE"],
                    }]
                },
                "validations": [{
                    "expression": "!object.metadata.name.startsWith('deny-')",
                    "message": "ConfigMap name cannot start with deny-",
                }]
            }
        });

        // Store the policy
        let policy_key = "/registry/validatingadmissionpolicies/deny-configmaps";
        storage
            .create::<serde_json::Value>(policy_key, &policy)
            .await
            .unwrap();

        // Create a binding for the policy (with old timestamp so it's "ready")
        let old_time = (chrono::Utc::now() - chrono::Duration::seconds(10)).to_rfc3339();
        let binding = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicyBinding",
            "metadata": {
                "name": "deny-configmaps-binding",
                "creationTimestamp": old_time,
            },
            "spec": {
                "policyName": "deny-configmaps",
                "validationActions": ["Deny"],
            }
        });

        let binding_key = "/registry/validatingadmissionpolicybindings/deny-configmaps-binding";
        storage
            .create::<serde_json::Value>(binding_key, &binding)
            .await
            .unwrap();

        // Test: Creating a configmap with name "deny-test" should be denied
        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "ConfigMap".to_string(),
        };
        let deny_cm = json!({
            "metadata": {"name": "deny-test", "namespace": "default"},
            "data": {"key": "value"},
        });

        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&deny_cm),
                None,
                Some("configmaps"),
                Some("default"),
            )
            .await;

        assert!(
            result.is_err(),
            "Should deny configmap with name starting with 'deny-'"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("ValidatingAdmissionPolicy"),
            "Error should mention VAP: {}",
            err_msg
        );

        // Test: Creating a configmap with a different name should be allowed
        let allow_cm = json!({
            "metadata": {"name": "allowed-cm", "namespace": "default"},
            "data": {"key": "value"},
        });

        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&allow_cm),
                None,
                Some("configmaps"),
                Some("default"),
            )
            .await;

        assert!(
            result.is_ok(),
            "Should allow configmap with name 'allowed-cm'"
        );
    }

    #[tokio::test]
    async fn test_vap_with_variables() {
        let storage = Arc::new(MemoryStorage::new());
        let manager = AdmissionWebhookManager::new(storage.clone());

        // Create a VAP that uses variables
        let policy = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicy",
            "metadata": {
                "name": "var-policy",
                "creationTimestamp": chrono::Utc::now().to_rfc3339(),
            },
            "spec": {
                "matchConstraints": {
                    "resourceRules": [{
                        "apiGroups": [""],
                        "resources": ["configmaps"],
                        "operations": ["CREATE"],
                    }]
                },
                "variables": [{
                    "name": "nameLen",
                    "expression": "size(object.metadata.name)",
                }],
                "validations": [{
                    "expression": "variables.nameLen <= 10",
                    "message": "Name too long",
                }]
            }
        });

        let policy_key = "/registry/validatingadmissionpolicies/var-policy";
        storage
            .create::<serde_json::Value>(policy_key, &policy)
            .await
            .unwrap();

        let old_time = (chrono::Utc::now() - chrono::Duration::seconds(10)).to_rfc3339();
        let binding = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicyBinding",
            "metadata": {
                "name": "var-policy-binding",
                "creationTimestamp": old_time,
            },
            "spec": {
                "policyName": "var-policy",
            }
        });

        let binding_key = "/registry/validatingadmissionpolicybindings/var-policy-binding";
        storage
            .create::<serde_json::Value>(binding_key, &binding)
            .await
            .unwrap();

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "ConfigMap".to_string(),
        };

        // Short name should pass
        let short_cm = json!({"metadata": {"name": "short"}});
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&short_cm),
                None,
                Some("configmaps"),
                Some("default"),
            )
            .await;
        assert!(result.is_ok(), "Short name should be allowed");

        // Long name should be denied
        let long_cm = json!({"metadata": {"name": "this-name-is-way-too-long"}});
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&long_cm),
                None,
                Some("configmaps"),
                Some("default"),
            )
            .await;
        assert!(result.is_err(), "Long name should be denied");
    }

    #[tokio::test]
    async fn test_vap_no_binding_skips_policy() {
        let storage = Arc::new(MemoryStorage::new());
        let manager = AdmissionWebhookManager::new(storage.clone());

        // Create a VAP without a binding
        let policy = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicy",
            "metadata": {
                "name": "unbound-policy",
                "creationTimestamp": chrono::Utc::now().to_rfc3339(),
            },
            "spec": {
                "matchConstraints": {
                    "resourceRules": [{
                        "apiGroups": [""],
                        "resources": ["configmaps"],
                        "operations": ["CREATE"],
                    }]
                },
                "validations": [{
                    "expression": "false",
                    "message": "Should never trigger",
                }]
            }
        });

        let policy_key = "/registry/validatingadmissionpolicies/unbound-policy";
        storage
            .create::<serde_json::Value>(policy_key, &policy)
            .await
            .unwrap();

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "ConfigMap".to_string(),
        };
        let cm = json!({"metadata": {"name": "test"}});

        // Should pass because there's no binding
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&cm),
                None,
                Some("configmaps"),
                Some("default"),
            )
            .await;
        assert!(result.is_ok(), "Should pass because no binding exists");
    }

    #[tokio::test]
    async fn test_vap_resource_mismatch_skips() {
        let storage = Arc::new(MemoryStorage::new());
        let manager = AdmissionWebhookManager::new(storage.clone());

        // Create a VAP that only matches pods
        let policy = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicy",
            "metadata": {
                "name": "pod-only",
                "creationTimestamp": chrono::Utc::now().to_rfc3339(),
            },
            "spec": {
                "matchConstraints": {
                    "resourceRules": [{
                        "apiGroups": [""],
                        "resources": ["pods"],
                        "operations": ["CREATE"],
                    }]
                },
                "validations": [{
                    "expression": "false",
                    "message": "Always deny",
                }]
            }
        });

        let policy_key = "/registry/validatingadmissionpolicies/pod-only";
        storage
            .create::<serde_json::Value>(policy_key, &policy)
            .await
            .unwrap();

        let old_time = (chrono::Utc::now() - chrono::Duration::seconds(10)).to_rfc3339();
        let binding = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicyBinding",
            "metadata": {
                "name": "pod-only-binding",
                "creationTimestamp": old_time,
            },
            "spec": {
                "policyName": "pod-only",
            }
        });

        let binding_key = "/registry/validatingadmissionpolicybindings/pod-only-binding";
        storage
            .create::<serde_json::Value>(binding_key, &binding)
            .await
            .unwrap();

        // Creating a configmap should NOT be denied (resource mismatch)
        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "ConfigMap".to_string(),
        };
        let cm = json!({"metadata": {"name": "test"}});

        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&cm),
                None,
                Some("configmaps"),
                Some("default"),
            )
            .await;
        assert!(
            result.is_ok(),
            "Should pass because resource type doesn't match"
        );
    }

    #[tokio::test]
    async fn test_vap_failure_policy_ignore() {
        let storage = Arc::new(MemoryStorage::new());
        let manager = AdmissionWebhookManager::new(storage.clone());

        // Create a VAP with Ignore failure policy and an expression that will error
        let policy = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicy",
            "metadata": {
                "name": "ignore-errors",
                "creationTimestamp": chrono::Utc::now().to_rfc3339(),
            },
            "spec": {
                "failurePolicy": "Ignore",
                "matchConstraints": {
                    "resourceRules": [{
                        "apiGroups": [""],
                        "resources": ["configmaps"],
                        "operations": ["CREATE"],
                    }]
                },
                "validations": [{
                    "expression": "object.nonexistent.field > 0",
                    "message": "Should not see this",
                }]
            }
        });

        let policy_key = "/registry/validatingadmissionpolicies/ignore-errors";
        storage
            .create::<serde_json::Value>(policy_key, &policy)
            .await
            .unwrap();

        let old_time = (chrono::Utc::now() - chrono::Duration::seconds(10)).to_rfc3339();
        let binding = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicyBinding",
            "metadata": {
                "name": "ignore-errors-binding",
                "creationTimestamp": old_time,
            },
            "spec": {
                "policyName": "ignore-errors",
            }
        });

        let binding_key = "/registry/validatingadmissionpolicybindings/ignore-errors-binding";
        storage
            .create::<serde_json::Value>(binding_key, &binding)
            .await
            .unwrap();

        let gvk = GroupVersionKind {
            group: "".to_string(),
            version: "v1".to_string(),
            kind: "ConfigMap".to_string(),
        };
        let cm = json!({"metadata": {"name": "test"}});

        // Should pass because failurePolicy is Ignore
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&cm),
                None,
                Some("configmaps"),
                Some("default"),
            )
            .await;
        assert!(
            result.is_ok(),
            "Should pass with Ignore failure policy on CEL error"
        );
    }

    /// Reproduces the K8s conformance test "should allow expressions to refer variables".
    /// The policy defines:
    ///   variables: [{name: "replicas", expression: "object.spec.replicas"},
    ///               {name: "oddReplicas", expression: "variables.replicas % 2 == 1"}]
    ///   validations: [{expression: "variables.replicas > 1"},
    ///                 {expression: "variables.oddReplicas"}]
    #[tokio::test]
    async fn test_vap_variables_refer_conformance() {
        let storage = Arc::new(MemoryStorage::new());
        let manager = AdmissionWebhookManager::new(storage.clone());

        let policy = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicy",
            "metadata": {"name": "var-refer-policy"},
            "spec": {
                "matchConstraints": {
                    "resourceRules": [{
                        "apiGroups": ["apps"],
                        "apiVersions": ["v1"],
                        "resources": ["deployments"],
                        "operations": ["CREATE"],
                    }]
                },
                "variables": [
                    {"name": "replicas", "expression": "object.spec.replicas"},
                    {"name": "oddReplicas", "expression": "variables.replicas % 2 == 1"},
                ],
                "validations": [
                    {"expression": "variables.replicas > 1"},
                    {"expression": "variables.oddReplicas"},
                ]
            }
        });

        let policy_key = "/registry/validatingadmissionpolicies/var-refer-policy";
        storage
            .create::<serde_json::Value>(policy_key, &policy)
            .await
            .unwrap();

        let binding = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicyBinding",
            "metadata": {"name": "var-refer-binding"},
            "spec": {
                "policyName": "var-refer-policy",
                "validationActions": ["Deny"],
            }
        });
        let binding_key = "/registry/validatingadmissionpolicybindings/var-refer-binding";
        storage
            .create::<serde_json::Value>(binding_key, &binding)
            .await
            .unwrap();

        let gvk = GroupVersionKind {
            group: "apps".to_string(),
            version: "v1".to_string(),
            kind: "Deployment".to_string(),
        };

        // 1-replica deployment should be denied (replicas > 1 fails)
        let deploy_1 = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {"name": "marker", "namespace": "default"},
            "spec": {
                "replicas": 1,
                "selector": {"matchLabels": {"app": "test"}},
                "template": {
                    "metadata": {"labels": {"app": "test"}},
                    "spec": {"containers": [{"name": "c", "image": "nginx"}]}
                }
            }
        });
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&deploy_1),
                None,
                Some("deployments"),
                Some("default"),
            )
            .await;
        assert!(result.is_err(), "1-replica deployment should be denied");

        // 3-replica deployment should be allowed (replicas > 1 AND oddReplicas both true)
        let deploy_3 = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {"name": "replicated", "namespace": "default"},
            "spec": {
                "replicas": 3,
                "selector": {"matchLabels": {"app": "test"}},
                "template": {
                    "metadata": {"labels": {"app": "test"}},
                    "spec": {"containers": [{"name": "c", "image": "nginx"}]}
                }
            }
        });
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&deploy_3),
                None,
                Some("deployments"),
                Some("default"),
            )
            .await;
        assert!(
            result.is_ok(),
            "3-replica deployment should be allowed: {:?}",
            result.err()
        );

        // ReplicaSet should NOT be matched (policy targets deployments only)
        let rs_gvk = GroupVersionKind {
            group: "apps".to_string(),
            version: "v1".to_string(),
            kind: "ReplicaSet".to_string(),
        };
        let rs = json!({
            "apiVersion": "apps/v1",
            "kind": "ReplicaSet",
            "metadata": {"name": "test-rs", "namespace": "default"},
            "spec": {"replicas": 1}
        });
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &rs_gvk,
                Some(&rs),
                None,
                Some("replicasets"),
                Some("default"),
            )
            .await;
        assert!(
            result.is_ok(),
            "ReplicaSet should not be matched by deployment policy"
        );
    }

    /// Reproduces the K8s conformance test "should validate against a Deployment".
    /// The policy uses namespaceObject.metadata.name in a validation expression.
    #[tokio::test]
    async fn test_vap_validate_deployment_with_namespace_object() {
        let storage = Arc::new(MemoryStorage::new());
        let manager = AdmissionWebhookManager::new(storage.clone());

        let ns_name = "test-ns-unique";

        // Create the namespace in storage so namespaceObject can be loaded
        let namespace_obj = json!({
            "apiVersion": "v1",
            "kind": "Namespace",
            "metadata": {
                "name": ns_name,
                "labels": {ns_name: "true"},
            }
        });
        let ns_key = format!("/registry/namespaces/{}", ns_name);
        storage
            .create::<serde_json::Value>(&ns_key, &namespace_obj)
            .await
            .unwrap();

        let policy = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicy",
            "metadata": {"name": "deploy-ns-policy"},
            "spec": {
                "matchConstraints": {
                    "resourceRules": [{
                        "apiGroups": ["apps"],
                        "apiVersions": ["v1"],
                        "resources": ["deployments"],
                        "operations": ["CREATE"],
                    }]
                },
                "validations": [
                    {"expression": "object.spec.replicas > 1", "messageExpression": "'wants replicas > 1, got ' + string(object.spec.replicas)"},
                    {"expression": format!("namespaceObject.metadata.name == '{}'", ns_name), "message": "Wrong namespace"},
                ]
            }
        });

        let policy_key = "/registry/validatingadmissionpolicies/deploy-ns-policy";
        storage
            .create::<serde_json::Value>(policy_key, &policy)
            .await
            .unwrap();

        let binding = json!({
            "apiVersion": "admissionregistration.k8s.io/v1",
            "kind": "ValidatingAdmissionPolicyBinding",
            "metadata": {"name": "deploy-ns-binding"},
            "spec": {
                "policyName": "deploy-ns-policy",
                "validationActions": ["Deny"],
            }
        });
        let binding_key = "/registry/validatingadmissionpolicybindings/deploy-ns-binding";
        storage
            .create::<serde_json::Value>(binding_key, &binding)
            .await
            .unwrap();

        let gvk = GroupVersionKind {
            group: "apps".to_string(),
            version: "v1".to_string(),
            kind: "Deployment".to_string(),
        };

        // 1-replica deployment: denied (fails replicas > 1)
        let deploy_1 = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {"name": "marker", "namespace": ns_name},
            "spec": {
                "replicas": 1,
                "selector": {"matchLabels": {"app": "test"}},
                "template": {
                    "metadata": {"labels": {"app": "test"}},
                    "spec": {"containers": [{"name": "c", "image": "nginx"}]}
                }
            }
        });
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&deploy_1),
                None,
                Some("deployments"),
                Some(ns_name),
            )
            .await;
        assert!(result.is_err(), "1-replica deployment should be denied");

        // 2-replica deployment in correct namespace: allowed
        let deploy_2 = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {"name": "replicated", "namespace": ns_name},
            "spec": {
                "replicas": 2,
                "selector": {"matchLabels": {"app": "test"}},
                "template": {
                    "metadata": {"labels": {"app": "test"}},
                    "spec": {"containers": [{"name": "c", "image": "nginx"}]}
                }
            }
        });
        let result = manager
            .run_validating_admission_policies_ext(
                &Operation::Create,
                &gvk,
                Some(&deploy_2),
                None,
                Some("deployments"),
                Some(ns_name),
            )
            .await;
        assert!(
            result.is_ok(),
            "2-replica deployment in correct namespace should be allowed: {:?}",
            result.err()
        );
    }
}

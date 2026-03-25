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

        debug!("Calling validating webhook {} at {}", webhook.name, url);

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

        debug!("Calling mutating webhook {} at {}", webhook.name, url);

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
            .build()
            .map_err(|e| {
                rusternetes_common::Error::Network(format!("Failed to create HTTP client: {}", e))
            })?;

        let response = client.post(url).json(review).send().await.map_err(|e| {
            rusternetes_common::Error::Network(format!("Webhook request failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(rusternetes_common::Error::Network(format!(
                "Webhook returned status: {}",
                response.status()
            )));
        }

        let review_response: AdmissionReview = response.json().await.map_err(|e| {
            rusternetes_common::Error::Network(format!("Failed to parse webhook response: {}", e))
        })?;

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
            // Build service URL
            let namespace = &service.namespace;
            let name = &service.name;
            let path = service.path.as_deref().unwrap_or("/");
            let port = service.port.unwrap_or(443);

            // In-cluster service URL
            let url = format!("https://{}.{}.svc:{}{}", name, namespace, port, path);

            return Ok(url);
        }

        Err(rusternetes_common::Error::InvalidResource(
            "Webhook client config must specify either url or service".to_string(),
        ))
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

                    // Skip webhooks whose service namespace no longer exists
                    if let Some(ref svc) = webhook.client_config.service {
                        let ns_key = rusternetes_storage::build_key("namespaces", None, &svc.namespace);
                        if self.storage.get::<serde_json::Value>(&ns_key).await.is_err() {
                            warn!("Skipping validating webhook {} — service namespace {} no longer exists", webhook.name, svc.namespace);
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
                    let response = self
                        .client
                        .call_validating_webhook(webhook, &request)
                        .await?;

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

                    // Skip webhooks whose service namespace no longer exists
                    if let Some(ref svc) = webhook.client_config.service {
                        let ns_key = rusternetes_storage::build_key("namespaces", None, &svc.namespace);
                        if self.storage.get::<serde_json::Value>(&ns_key).await.is_err() {
                            warn!("Skipping webhook {} — service namespace {} no longer exists", webhook.name, svc.namespace);
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

                    // Call the webhook
                    let response = self.client.call_mutating_webhook(webhook, &request).await?;

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
}

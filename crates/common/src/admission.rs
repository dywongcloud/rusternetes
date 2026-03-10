// Admission control for Kubernetes API requests
//
// This module provides admission controllers that intercept API requests
// before they are persisted to etcd. Admission controllers can:
// - Validate requests (ValidatingAdmissionController)
// - Mutate requests (MutatingAdmissionController)
// - Both validate and mutate (MutatingAdmissionWebhook, ValidatingAdmissionWebhook)

use crate::resources::pod::Pod;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Result of an admission decision
#[derive(Debug, Clone, PartialEq)]
pub enum AdmissionResponse {
    /// Request is allowed
    Allow,
    /// Request is denied with a reason
    Deny(String),
    /// Request is allowed with mutations (JSON Patch format)
    AllowWithPatch(Vec<PatchOperation>),
}

impl AdmissionResponse {
    pub fn is_allowed(&self) -> bool {
        matches!(self, AdmissionResponse::Allow | AdmissionResponse::AllowWithPatch(_))
    }

    pub fn deny_reason(&self) -> Option<&str> {
        match self {
            AdmissionResponse::Deny(reason) => Some(reason),
            _ => None,
        }
    }

    pub fn patches(&self) -> Option<&[PatchOperation]> {
        match self {
            AdmissionResponse::AllowWithPatch(patches) => Some(patches),
            _ => None,
        }
    }
}

/// JSON Patch operation (RFC 6902)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchOperation {
    pub op: PatchOp,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchOp {
    Add,
    Remove,
    Replace,
    Move,
    Copy,
    Test,
}

/// Admission request containing the resource to be validated/mutated
#[derive(Debug, Clone)]
pub struct AdmissionRequest {
    /// The operation being performed (CREATE, UPDATE, DELETE)
    pub operation: Operation,
    /// The resource kind (Pod, Service, etc.)
    pub kind: String,
    /// The resource namespace (if namespaced)
    pub namespace: Option<String>,
    /// The resource name
    pub name: String,
    /// The resource object (JSON)
    pub object: serde_json::Value,
    /// The old resource object for UPDATE operations
    pub old_object: Option<serde_json::Value>,
    /// The user making the request
    pub user_info: UserInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Operation {
    Create,
    Update,
    Delete,
    Connect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub uid: String,
    pub groups: Vec<String>,
}

/// Trait for all admission controllers
#[async_trait]
pub trait AdmissionController: Send + Sync {
    /// Name of the admission controller
    fn name(&self) -> &str;

    /// Admit a request, returning whether to allow it and any patches
    async fn admit(&self, request: &AdmissionRequest) -> AdmissionResponse;

    /// Whether this controller supports the given operation
    fn supports_operation(&self, operation: &Operation) -> bool {
        matches!(operation, Operation::Create | Operation::Update)
    }
}

/// Chain of admission controllers
pub struct AdmissionChain {
    controllers: Vec<Arc<dyn AdmissionController>>,
}

impl AdmissionChain {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
        }
    }

    pub fn with_controller(mut self, controller: Arc<dyn AdmissionController>) -> Self {
        self.controllers.push(controller);
        self
    }

    pub fn with_built_in_controllers(self) -> Self {
        self
            .with_controller(Arc::new(NamespaceLifecycleController))
            .with_controller(Arc::new(ResourceQuotaController))
            .with_controller(Arc::new(LimitRangerController))
            .with_controller(Arc::new(PodSecurityStandardsController))
    }

    /// Run all admission controllers in the chain
    pub async fn admit(&self, request: &AdmissionRequest) -> AdmissionResponse {
        let mut all_patches = Vec::new();

        for controller in &self.controllers {
            // Skip controllers that don't support this operation
            if !controller.supports_operation(&request.operation) {
                continue;
            }

            let response = controller.admit(request).await;

            match response {
                AdmissionResponse::Deny(reason) => {
                    return AdmissionResponse::Deny(format!(
                        "{}: {}",
                        controller.name(),
                        reason
                    ));
                }
                AdmissionResponse::AllowWithPatch(mut patches) => {
                    all_patches.append(&mut patches);
                }
                AdmissionResponse::Allow => {}
            }
        }

        if all_patches.is_empty() {
            AdmissionResponse::Allow
        } else {
            AdmissionResponse::AllowWithPatch(all_patches)
        }
    }
}

impl Default for AdmissionChain {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Built-in Admission Controllers =====

/// NamespaceLifecycle prevents creating resources in non-existent or terminating namespaces
pub struct NamespaceLifecycleController;

#[async_trait]
impl AdmissionController for NamespaceLifecycleController {
    fn name(&self) -> &str {
        "NamespaceLifecycle"
    }

    async fn admit(&self, request: &AdmissionRequest) -> AdmissionResponse {
        // Only check namespaced resources
        if let Some(namespace) = &request.namespace {
            // Special namespaces are always allowed
            if namespace == "kube-system" || namespace == "kube-public" || namespace == "default" {
                return AdmissionResponse::Allow;
            }

            // For CREATE operations, check if namespace exists
            // In real implementation, this would query etcd
            // For now, we'll allow it
        }

        AdmissionResponse::Allow
    }
}

/// ResourceQuota enforces resource consumption limits per namespace
pub struct ResourceQuotaController;

#[async_trait]
impl AdmissionController for ResourceQuotaController {
    fn name(&self) -> &str {
        "ResourceQuota"
    }

    async fn admit(&self, _request: &AdmissionRequest) -> AdmissionResponse {
        // TODO: Implement resource quota checking
        // For now, always allow
        AdmissionResponse::Allow
    }
}

/// LimitRanger enforces min/max resource limits and provides defaults
pub struct LimitRangerController;

#[async_trait]
impl AdmissionController for LimitRangerController {
    fn name(&self) -> &str {
        "LimitRanger"
    }

    async fn admit(&self, _request: &AdmissionRequest) -> AdmissionResponse {
        // TODO: Implement limit range checking
        // For now, always allow
        AdmissionResponse::Allow
    }
}

/// PodSecurityStandards enforces Pod Security Standards (restricted, baseline, privileged)
pub struct PodSecurityStandardsController;

#[async_trait]
impl AdmissionController for PodSecurityStandardsController {
    fn name(&self) -> &str {
        "PodSecurityStandards"
    }

    async fn admit(&self, request: &AdmissionRequest) -> AdmissionResponse {
        // Only check Pod resources
        if request.kind != "Pod" {
            return AdmissionResponse::Allow;
        }

        // Parse the pod from the request object
        let pod: Pod = match serde_json::from_value(request.object.clone()) {
            Ok(p) => p,
            Err(e) => return AdmissionResponse::Deny(format!("Failed to parse Pod: {}", e)),
        };

        // Get namespace security level (default to baseline)
        // In real implementation, this would read from namespace labels
        let security_level = PodSecurityLevel::Baseline;

        // Check pod against security level
        if let Err(violations) = check_pod_security(&pod, &security_level) {
            return AdmissionResponse::Deny(format!(
                "Pod violates {} security standard: {}",
                security_level.as_str(),
                violations.join(", ")
            ));
        }

        AdmissionResponse::Allow
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PodSecurityLevel {
    Privileged, // Unrestricted
    Baseline,   // Minimally restrictive
    Restricted, // Heavily restricted
}

impl PodSecurityLevel {
    pub fn as_str(&self) -> &str {
        match self {
            PodSecurityLevel::Privileged => "privileged",
            PodSecurityLevel::Baseline => "baseline",
            PodSecurityLevel::Restricted => "restricted",
        }
    }
}

/// Check a pod against a security level
fn check_pod_security(pod: &Pod, level: &PodSecurityLevel) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();

    match level {
        PodSecurityLevel::Privileged => {
            // Allow everything
            return Ok(());
        }
        PodSecurityLevel::Baseline => {
            // Baseline: disallow known privilege escalations
            if let Some(spec) = &pod.spec {
                // Check hostNetwork, hostPID, hostIPC
                if spec.host_network.unwrap_or(false) {
                    violations.push("hostNetwork=true not allowed in baseline mode".to_string());
                }
                if spec.host_pid.unwrap_or(false) {
                    violations.push("hostPID=true not allowed in baseline mode".to_string());
                }
                if spec.host_ipc.unwrap_or(false) {
                    violations.push("hostIPC=true not allowed in baseline mode".to_string());
                }

                // Check privileged containers
                for container in &spec.containers {
                    if let Some(security_context) = &container.security_context {
                        if security_context.privileged.unwrap_or(false) {
                            violations.push(format!(
                                "Container '{}' has privileged=true not allowed in baseline mode",
                                container.name
                            ));
                        }

                        // Check capabilities
                        if let Some(caps) = &security_context.capabilities {
                            if let Some(add) = &caps.add {
                                let disallowed: Vec<_> = add
                                    .iter()
                                    .filter(|c| !is_allowed_capability_baseline(c))
                                    .collect();
                                if !disallowed.is_empty() {
                                    violations.push(format!(
                                        "Container '{}' adds disallowed capabilities: {:?}",
                                        container.name, disallowed
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        PodSecurityLevel::Restricted => {
            // Restricted: most restrictive policy
            if let Some(spec) = &pod.spec {
                // Disallow host namespaces
                if spec.host_network.unwrap_or(false) {
                    violations.push("hostNetwork=true not allowed in restricted mode".to_string());
                }
                if spec.host_pid.unwrap_or(false) {
                    violations.push("hostPID=true not allowed in restricted mode".to_string());
                }
                if spec.host_ipc.unwrap_or(false) {
                    violations.push("hostIPC=true not allowed in restricted mode".to_string());
                }

                // All containers must run as non-root
                for container in &spec.containers {
                    if let Some(security_context) = &container.security_context {
                        if security_context.privileged.unwrap_or(false) {
                            violations.push(format!(
                                "Container '{}' must not be privileged",
                                container.name
                            ));
                        }

                        if security_context.run_as_non_root != Some(true) {
                            violations.push(format!(
                                "Container '{}' must set runAsNonRoot=true",
                                container.name
                            ));
                        }

                        // No privilege escalation
                        if security_context.allow_privilege_escalation.unwrap_or(true) {
                            violations.push(format!(
                                "Container '{}' must set allowPrivilegeEscalation=false",
                                container.name
                            ));
                        }

                        // Must drop ALL capabilities
                        if let Some(caps) = &security_context.capabilities {
                            if caps.drop.as_ref().map_or(true, |d| !d.contains(&"ALL".to_string())) {
                                violations.push(format!(
                                    "Container '{}' must drop ALL capabilities",
                                    container.name
                                ));
                            }
                        } else {
                            violations.push(format!(
                                "Container '{}' must drop ALL capabilities",
                                container.name
                            ));
                        }

                        // Seccomp profile required
                        if security_context.seccomp_profile.is_none() {
                            violations.push(format!(
                                "Container '{}' must define seccomp profile",
                                container.name
                            ));
                        }
                    } else {
                        violations.push(format!(
                            "Container '{}' must define securityContext",
                            container.name
                        ));
                    }
                }

                // Volume types restrictions
                if let Some(volumes) = &spec.volumes {
                    for volume in volumes {
                        // Check for hostPath volumes
                        if volume.host_path.is_some() {
                            violations.push(format!(
                                "Volume '{}' uses hostPath which is not allowed in restricted mode",
                                volume.name
                            ));
                        }
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Check if a capability is allowed in baseline mode
fn is_allowed_capability_baseline(cap: &str) -> bool {
    matches!(
        cap.to_uppercase().as_str(),
        "AUDIT_WRITE"
            | "CHOWN"
            | "DAC_OVERRIDE"
            | "FOWNER"
            | "FSETID"
            | "KILL"
            | "MKNOD"
            | "NET_BIND_SERVICE"
            | "SETFCAP"
            | "SETGID"
            | "SETPCAP"
            | "SETUID"
            | "SYS_CHROOT"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admission_response() {
        let allow = AdmissionResponse::Allow;
        assert!(allow.is_allowed());
        assert!(allow.deny_reason().is_none());
        assert!(allow.patches().is_none());

        let deny = AdmissionResponse::Deny("test reason".to_string());
        assert!(!deny.is_allowed());
        assert_eq!(deny.deny_reason(), Some("test reason"));

        let patch = AdmissionResponse::AllowWithPatch(vec![]);
        assert!(patch.is_allowed());
        assert_eq!(patch.patches(), Some(&[][..]));
    }

    #[test]
    fn test_pod_security_levels() {
        assert_eq!(PodSecurityLevel::Privileged.as_str(), "privileged");
        assert_eq!(PodSecurityLevel::Baseline.as_str(), "baseline");
        assert_eq!(PodSecurityLevel::Restricted.as_str(), "restricted");
    }

    #[test]
    fn test_allowed_capabilities() {
        assert!(is_allowed_capability_baseline("NET_BIND_SERVICE"));
        assert!(is_allowed_capability_baseline("CHOWN"));
        assert!(!is_allowed_capability_baseline("SYS_ADMIN"));
        assert!(!is_allowed_capability_baseline("NET_ADMIN"));
    }
}

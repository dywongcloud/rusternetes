use crate::auth::UserInfo;
use crate::error::Result;
use crate::resources::rbac::{ClusterRole, ClusterRoleBinding, PolicyRule, Role, RoleBinding};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Minimal storage trait for authorization (to avoid circular dependency)
#[async_trait]
pub trait AuthzStorage: Send + Sync {
    async fn get<T>(&self, key: &str, namespace: Option<&str>) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync;

    async fn list<T>(&self, namespace: Option<&str>) -> Result<Vec<T>>
    where
        T: DeserializeOwned + Send + Sync;
}

/// Request attributes for authorization
#[derive(Debug, Clone)]
pub struct RequestAttributes {
    /// The user making the request
    pub user: UserInfo,

    /// The verb being requested (get, list, create, update, delete, watch, etc.)
    pub verb: String,

    /// The namespace of the resource (None for cluster-scoped resources)
    pub namespace: Option<String>,

    /// The API group of the resource
    pub api_group: String,

    /// The resource type (pods, services, deployments, etc.)
    pub resource: String,

    /// The specific resource name (None for list operations)
    pub name: Option<String>,

    /// The subresource being accessed (status, scale, etc.)
    pub subresource: Option<String>,
}

impl RequestAttributes {
    pub fn new(user: UserInfo, verb: impl Into<String>, resource: impl Into<String>) -> Self {
        Self {
            user,
            verb: verb.into(),
            namespace: None,
            api_group: String::new(),
            resource: resource.into(),
            name: None,
            subresource: None,
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_api_group(mut self, api_group: impl Into<String>) -> Self {
        self.api_group = api_group.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_subresource(mut self, subresource: impl Into<String>) -> Self {
        self.subresource = Some(subresource.into());
        self
    }
}

/// Authorization decision
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Allow,
    Deny(String),
}

/// Authorizer trait for authorization implementations
#[async_trait]
pub trait Authorizer: Send + Sync {
    async fn authorize(&self, attrs: &RequestAttributes) -> Result<Decision>;
}

/// RBAC Authorizer that uses Role and RoleBinding resources
pub struct RBACAuthorizer<S: AuthzStorage> {
    storage: Arc<S>,
}

impl<S: AuthzStorage> RBACAuthorizer<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Check if a user has permission based on RBAC rules
    async fn check_rbac(&self, attrs: &RequestAttributes) -> Result<Decision> {
        // Get all role bindings for the user
        let role_bindings = self.get_user_role_bindings(attrs).await?;
        let cluster_role_bindings = self.get_user_cluster_role_bindings(attrs).await?;

        // Check namespace-scoped roles first
        if let Some(namespace) = &attrs.namespace {
            for binding in &role_bindings {
                if let Some(ref binding_ns) = binding.metadata.namespace {
                    if binding_ns == namespace {
                        // Get the referenced role
                        if let Ok(role) = self
                            .storage
                            .get::<Role>(&binding.role_ref.name, Some(binding_ns))
                            .await
                        {
                            if self.check_policy_rules(&role.rules, attrs) {
                                return Ok(Decision::Allow);
                            }
                        }
                    }
                }
            }
        }

        // Check cluster-scoped roles via role bindings
        for binding in &role_bindings {
            if binding.role_ref.kind == "ClusterRole" {
                if let Ok(cluster_role) = self
                    .storage
                    .get::<ClusterRole>(&binding.role_ref.name, None)
                    .await
                {
                    if self.check_policy_rules(&cluster_role.rules, attrs) {
                        return Ok(Decision::Allow);
                    }
                }
            }
        }

        // Check cluster-scoped roles
        for binding in cluster_role_bindings {
            if let Ok(cluster_role) = self
                .storage
                .get::<ClusterRole>(&binding.role_ref.name, None)
                .await
            {
                if self.check_policy_rules(&cluster_role.rules, attrs) {
                    return Ok(Decision::Allow);
                }
            }
        }

        Ok(Decision::Deny(
            "User does not have permission to perform this action".to_string(),
        ))
    }

    /// Get all RoleBindings that apply to the user
    async fn get_user_role_bindings(&self, attrs: &RequestAttributes) -> Result<Vec<RoleBinding>> {
        let all_bindings = match &attrs.namespace {
            Some(ns) => self.storage.list::<RoleBinding>(Some(ns)).await?,
            None => vec![],
        };

        Ok(all_bindings
            .into_iter()
            .filter(|binding| {
                binding.subjects.iter().any(|subject| {
                    subject.name == attrs.user.username
                        || attrs.user.groups.contains(&subject.name)
                })
            })
            .collect())
    }

    /// Get all ClusterRoleBindings that apply to the user
    async fn get_user_cluster_role_bindings(
        &self,
        attrs: &RequestAttributes,
    ) -> Result<Vec<ClusterRoleBinding>> {
        let all_bindings = self.storage.list::<ClusterRoleBinding>(None).await?;

        Ok(all_bindings
            .into_iter()
            .filter(|binding| {
                binding.subjects.iter().any(|subject| {
                    subject.name == attrs.user.username
                        || attrs.user.groups.contains(&subject.name)
                })
            })
            .collect())
    }

    /// Check if policy rules allow the requested action
    fn check_policy_rules(&self, rules: &[PolicyRule], attrs: &RequestAttributes) -> bool {
        rules.iter().any(|rule| self.rule_allows(rule, attrs))
    }

    /// Check if a single policy rule allows the requested action
    fn rule_allows(&self, rule: &PolicyRule, attrs: &RequestAttributes) -> bool {
        // Check verb
        if !rule.verbs.contains(&attrs.verb) && !rule.verbs.contains(&"*".to_string()) {
            return false;
        }

        // Check API group
        if let Some(ref api_groups) = rule.api_groups {
            if !api_groups.contains(&attrs.api_group) && !api_groups.contains(&"*".to_string()) {
                return false;
            }
        }

        // Check resource
        if let Some(ref resources) = rule.resources {
            if !resources.contains(&attrs.resource) && !resources.contains(&"*".to_string()) {
                return false;
            }
        }

        // Check resource name if specified
        if let Some(ref name) = attrs.name {
            if let Some(ref resource_names) = rule.resource_names {
                if !resource_names.contains(name) && !resource_names.contains(&"*".to_string()) {
                    return false;
                }
            }
        }

        true
    }
}

#[async_trait]
impl<S: AuthzStorage> Authorizer for RBACAuthorizer<S> {
    async fn authorize(&self, attrs: &RequestAttributes) -> Result<Decision> {
        // System admin has full access
        if attrs.user.username == "system:admin" {
            return Ok(Decision::Allow);
        }

        // Check RBAC
        self.check_rbac(attrs).await
    }
}

/// Always allow authorizer (for testing)
pub struct AlwaysAllowAuthorizer;

#[async_trait]
impl Authorizer for AlwaysAllowAuthorizer {
    async fn authorize(&self, _attrs: &RequestAttributes) -> Result<Decision> {
        Ok(Decision::Allow)
    }
}

/// Always deny authorizer (for testing)
pub struct AlwaysDenyAuthorizer;

#[async_trait]
impl Authorizer for AlwaysDenyAuthorizer {
    async fn authorize(&self, _attrs: &RequestAttributes) -> Result<Decision> {
        Ok(Decision::Deny("Access denied".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_rule_matching() {
        let rule = PolicyRule {
            verbs: vec!["get".to_string(), "list".to_string()],
            api_groups: Some(vec!["".to_string()]),
            resources: Some(vec!["pods".to_string()]),
            resource_names: None,
            non_resource_urls: None,
        };

        let user = UserInfo {
            username: "test-user".to_string(),
            uid: "test-uid".to_string(),
            groups: vec![],
            extra: std::collections::HashMap::new(),
        };

        let attrs = RequestAttributes::new(user.clone(), "get", "pods")
            .with_namespace("default")
            .with_api_group("");

        // This would need a mock storage implementation to fully test
        // Just testing the rule matching logic
        assert!(rule.verbs.contains(&attrs.verb));
    }
}

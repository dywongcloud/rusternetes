use crate::client::ApiClient;
use crate::types::AuthCommands;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelfSubjectAccessReview {
    api_version: String,
    kind: String,
    spec: SelfSubjectAccessReviewSpec,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelfSubjectAccessReviewSpec {
    resource_attributes: Option<ResourceAttributes>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceAttributes {
    namespace: Option<String>,
    verb: String,
    resource: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct SelfSubjectAccessReviewStatus {
    allowed: bool,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Deserialize)]
struct SelfSubjectAccessReviewResponse {
    status: SelfSubjectAccessReviewStatus,
}

/// Execute auth commands
pub async fn execute(
    client: &ApiClient,
    command: AuthCommands,
    default_namespace: &str,
) -> Result<()> {
    match command {
        AuthCommands::CanI {
            verb,
            resource,
            name,
            namespace,
            all_namespaces,
        } => {
            let ns = if all_namespaces {
                None
            } else {
                Some(
                    namespace
                        .as_deref()
                        .unwrap_or(default_namespace)
                        .to_string(),
                )
            };

            let review = SelfSubjectAccessReview {
                api_version: "authorization.k8s.io/v1".to_string(),
                kind: "SelfSubjectAccessReview".to_string(),
                spec: SelfSubjectAccessReviewSpec {
                    resource_attributes: Some(ResourceAttributes {
                        namespace: ns,
                        verb,
                        resource,
                        name,
                    }),
                },
            };

            let response: SelfSubjectAccessReviewResponse = client
                .post(
                    "/apis/authorization.k8s.io/v1/selfsubjectaccessreviews",
                    &review,
                )
                .await
                .context("Failed to check authorization")?;

            if response.status.allowed {
                println!("yes");
                if let Some(reason) = response.status.reason {
                    println!("Reason: {}", reason);
                }
            } else {
                println!("no");
                if let Some(reason) = response.status.reason {
                    println!("Reason: {}", reason);
                }
            }
        }
        AuthCommands::Whoami { output } => {
            // Use SelfSubjectReview (k8s 1.27+) or fallback to token review
            let result: Value = client
                .post(
                    "/apis/authentication.k8s.io/v1/selfsubjectreviews",
                    &serde_json::json!({
                        "apiVersion": "authentication.k8s.io/v1",
                        "kind": "SelfSubjectReview",
                    }),
                )
                .await
                .context("Failed to get user identity")?;

            if let Some(fmt) = output {
                match fmt.as_str() {
                    "json" => println!("{}", serde_json::to_string_pretty(&result)?),
                    "yaml" => println!("{}", serde_yaml::to_string(&result)?),
                    _ => println!("{}", serde_json::to_string_pretty(&result)?),
                }
            } else {
                // Extract user info for simple output
                if let Some(user_info) = result.get("status").and_then(|s| s.get("userInfo")) {
                    if let Some(username) = user_info.get("username") {
                        println!("Username: {}", username);
                    }
                    if let Some(uid) = user_info.get("uid") {
                        println!("UID: {}", uid);
                    }
                    if let Some(groups) = user_info.get("groups") {
                        println!("Groups: {}", groups);
                    }
                }
            }
        }
    }

    Ok(())
}

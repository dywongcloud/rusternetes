use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{
        SelfSubjectReview, SelfSubjectReviewStatus, TokenRequest, TokenRequestStatus, TokenReview,
        TokenReviewStatus, UserInfo,
    },
    Result,
};
use rusternetes_storage::Storage;
use std::sync::Arc;
use tracing::info;

/// Create a TokenReview (authentication.k8s.io/v1)
/// TokenReview attempts to authenticate a token to a known user.
pub async fn create_token_review(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(mut token_review): Json<TokenReview>,
) -> Result<Json<TokenReview>> {
    info!("Creating token review");

    // Check authorization - creating a TokenReview requires impersonation privileges
    let attrs = RequestAttributes::new(auth_ctx.user.clone(), "create", "tokenreviews")
        .with_api_group("authentication.k8s.io");

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Authenticate the provided token using the available authentication mechanisms
    // Try service account token first
    let status = if let Ok(claims) = state.token_manager.validate_token(&token_review.spec.token) {
        // Valid service account token
        TokenReviewStatus {
            authenticated: Some(true),
            user: Some(UserInfo {
                username: Some(format!(
                    "system:serviceaccount:{}:{}",
                    claims.namespace, claims.sub
                )),
                uid: Some(claims.uid),
                groups: Some(vec![
                    "system:serviceaccounts".to_string(),
                    format!("system:serviceaccounts:{}", claims.namespace),
                ]),
                extra: None,
            }),
            audiences: token_review.spec.audiences.clone(),
            error: None,
        }
    } else {
        // Token validation failed - could be bootstrap token, OIDC, or invalid
        // For now, mark as unauthenticated
        // Note: Full implementation would try other auth mechanisms here
        // (bootstrap tokens, OIDC, webhook auth, etc.)
        TokenReviewStatus {
            authenticated: Some(false),
            user: None,
            audiences: None,
            error: Some(
                "Token authentication failed - not a valid service account token".to_string(),
            ),
        }
    };

    token_review.status = Some(status);
    Ok(Json(token_review))
}

/// Create a TokenRequest (authentication.k8s.io/v1)
/// TokenRequest requests a token for a given service account.
pub async fn create_token_request(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, service_account_name)): Path<(String, String)>,
    Json(mut token_request): Json<TokenRequest>,
) -> Result<Json<TokenRequest>> {
    info!(
        "Creating token request for service account {}/{}",
        namespace, service_account_name
    );

    // Check authorization - requires permission to create token requests for the service account
    let attrs = RequestAttributes::new(auth_ctx.user.clone(), "create", "serviceaccounts/token")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&service_account_name);

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Verify the service account exists
    let sa_key = format!(
        "/api/v1/namespaces/{}/serviceaccounts/{}",
        namespace, service_account_name
    );
    let sa: rusternetes_common::resources::ServiceAccount = state.storage.get(&sa_key).await?;

    // Calculate expiration time
    let expiration_seconds = token_request.spec.expiration_seconds.unwrap_or(3600);
    let expiration_hours = expiration_seconds / 3600;

    let expiration_timestamp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(expiration_seconds))
        .ok_or_else(|| {
            rusternetes_common::Error::Internal("Failed to calculate expiration time".to_string())
        })?;

    // Generate a proper JWT service account token
    let mut claims = rusternetes_common::auth::ServiceAccountClaims::new(
        service_account_name.clone(),
        namespace.clone(),
        sa.metadata.uid.clone(),
        expiration_hours,
    );

    // Set audience from the request (TokenRequestSpec.audiences is Vec<String>, not Option)
    if !token_request.spec.audiences.is_empty() {
        claims.aud = token_request.spec.audiences.clone();
    }

    let token = state.token_manager.generate_token(claims)?;

    token_request.status = Some(TokenRequestStatus {
        token,
        expiration_timestamp: expiration_timestamp.to_rfc3339(),
    });

    Ok(Json(token_request))
}

/// Create a SelfSubjectReview (authentication.k8s.io/v1)
/// SelfSubjectReview contains the user information that the kube-apiserver has about the user making this request.
pub async fn create_self_subject_review(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(mut self_subject_review): Json<SelfSubjectReview>,
) -> Result<Json<SelfSubjectReview>> {
    info!("Creating self subject review for user: {:?}", auth_ctx.user);

    // Check authorization - creating a SelfSubjectReview is always allowed
    let attrs = RequestAttributes::new(auth_ctx.user.clone(), "create", "selfsubjectreviews")
        .with_api_group("authentication.k8s.io");

    if let Decision::Deny(reason) = state.authorizer.authorize(&attrs).await? {
        return Err(rusternetes_common::Error::Forbidden(reason));
    }

    // Return the current user's information
    self_subject_review.status = Some(SelfSubjectReviewStatus {
        user_info: Some(UserInfo {
            username: Some(auth_ctx.user.username.clone()),
            uid: Some(auth_ctx.user.uid.clone()),
            groups: Some(auth_ctx.user.groups.clone()),
            extra: Some(auth_ctx.user.extra.clone()),
        }),
    });

    Ok(Json(self_subject_review))
}

#[cfg(test)]
#[cfg(feature = "integration-tests")] // Disable incomplete tests
mod tests {
    use super::*;
    use crate::state::MockAuth;
    use rusternetes_common::resources::TokenReviewSpec;

    #[tokio::test]
    async fn test_token_review_authenticated() {
        // This test would verify that a valid token returns authenticated=true
        // Implementation would depend on the actual auth system
    }

    #[tokio::test]
    async fn test_token_review_unauthenticated() {
        // This test would verify that an invalid token returns authenticated=false
    }

    #[tokio::test]
    async fn test_self_subject_review() {
        // This test would verify that SelfSubjectReview returns the current user's info
    }
}

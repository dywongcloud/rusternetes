use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use rusternetes_common::auth::{BootstrapTokenManager, TokenManager, UserInfo};
use std::sync::Arc;
use tracing::{debug, warn};

/// Extension type to carry UserInfo through the request
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user: UserInfo,
}

/// Middleware that adds a default admin AuthContext when skip_auth is enabled
pub async fn skip_auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Create an admin user context
    let admin_user = UserInfo {
        username: "admin".to_string(),
        uid: "system:admin".to_string(),
        groups: vec!["system:masters".to_string()],
        extra: std::collections::HashMap::new(),
    };

    // Insert AuthContext into request extensions
    request.extensions_mut().insert(AuthContext { user: admin_user });

    Ok(next.run(request).await)
}

/// Authentication middleware that extracts and validates JWT tokens
pub async fn auth_middleware(
    Extension(token_manager): Extension<Arc<TokenManager>>,
    Extension(bootstrap_token_manager): Extension<Arc<BootstrapTokenManager>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract Bearer token from Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let user = if auth_header.starts_with("Bearer ") {
        let token = &auth_header[7..]; // Skip "Bearer "

        // Try to validate as a service account token first
        if let Ok(claims) = token_manager.validate_token(token) {
            let user_info = UserInfo::from_service_account_claims(&claims);
            debug!("Authenticated user (service account): {}", user_info.username);
            user_info
        }
        // Try to validate as a bootstrap token
        else if let Ok(bootstrap_token) = bootstrap_token_manager.validate_token(token) {
            let user_info = UserInfo::from_bootstrap_token(&bootstrap_token);
            debug!("Authenticated user (bootstrap token): {}", user_info.username);
            user_info
        }
        // Invalid token
        else {
            warn!("Invalid token");
            return Err((StatusCode::UNAUTHORIZED, "Invalid token").into_response());
        }
    } else {
        // Anonymous user
        debug!("Anonymous request");
        UserInfo::anonymous()
    };

    // Insert UserInfo into request extensions
    request.extensions_mut().insert(AuthContext { user });

    Ok(next.run(request).await)
}

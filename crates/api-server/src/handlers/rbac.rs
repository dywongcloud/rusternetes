use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

// Role handlers
pub async fn create_role(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut role): Json<Role>,
) -> Result<(StatusCode, Json<Role>)> {
    info!("Creating role: {}/{}", namespace, role.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "roles")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    role.metadata.namespace = Some(namespace.clone());

    let key = build_key("roles", Some(&namespace), &role.metadata.name);
    let created = state.storage.create(&key, &role).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_role(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Role>> {
    info!("Getting role: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "roles")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("roles", Some(&namespace), &name);
    let role = state.storage.get(&key).await?;

    Ok(Json(role))
}

pub async fn update_role(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut role): Json<Role>,
) -> Result<Json<Role>> {
    info!("Updating role: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "roles")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    role.metadata.name = name.clone();
    role.metadata.namespace = Some(namespace.clone());

    let key = build_key("roles", Some(&namespace), &name);
    let updated = state.storage.update(&key, &role).await?;

    Ok(Json(updated))
}

pub async fn delete_role(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting role: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "roles")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("roles", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_roles(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<Role>>> {
    info!("Listing roles in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "roles")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("roles", Some(&namespace));
    let roles = state.storage.list(&prefix).await?;

    Ok(Json(roles))
}

// RoleBinding handlers
pub async fn create_rolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut rolebinding): Json<RoleBinding>,
) -> Result<(StatusCode, Json<RoleBinding>)> {
    info!("Creating rolebinding: {}/{}", namespace, rolebinding.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "rolebindings")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    rolebinding.metadata.namespace = Some(namespace.clone());

    let key = build_key("rolebindings", Some(&namespace), &rolebinding.metadata.name);
    let created = state.storage.create(&key, &rolebinding).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_rolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<RoleBinding>> {
    info!("Getting rolebinding: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "rolebindings")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("rolebindings", Some(&namespace), &name);
    let rolebinding = state.storage.get(&key).await?;

    Ok(Json(rolebinding))
}

pub async fn update_rolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut rolebinding): Json<RoleBinding>,
) -> Result<Json<RoleBinding>> {
    info!("Updating rolebinding: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "rolebindings")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    rolebinding.metadata.name = name.clone();
    rolebinding.metadata.namespace = Some(namespace.clone());

    let key = build_key("rolebindings", Some(&namespace), &name);
    let updated = state.storage.update(&key, &rolebinding).await?;

    Ok(Json(updated))
}

pub async fn delete_rolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting rolebinding: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "rolebindings")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("rolebindings", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_rolebindings(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<RoleBinding>>> {
    info!("Listing rolebindings in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "rolebindings")
        .with_namespace(&namespace)
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("rolebindings", Some(&namespace));
    let rolebindings = state.storage.list(&prefix).await?;

    Ok(Json(rolebindings))
}

// ClusterRole handlers
pub async fn create_clusterrole(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(clusterrole): Json<ClusterRole>,
) -> Result<(StatusCode, Json<ClusterRole>)> {
    info!("Creating clusterrole: {}", clusterrole.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "clusterroles")
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("clusterroles", None, &clusterrole.metadata.name);
    let created = state.storage.create(&key, &clusterrole).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_clusterrole(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<ClusterRole>> {
    info!("Getting clusterrole: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "clusterroles")
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("clusterroles", None, &name);
    let clusterrole = state.storage.get(&key).await?;

    Ok(Json(clusterrole))
}

pub async fn update_clusterrole(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut clusterrole): Json<ClusterRole>,
) -> Result<Json<ClusterRole>> {
    info!("Updating clusterrole: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "clusterroles")
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    clusterrole.metadata.name = name.clone();

    let key = build_key("clusterroles", None, &name);
    let updated = state.storage.update(&key, &clusterrole).await?;

    Ok(Json(updated))
}

pub async fn delete_clusterrole(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting clusterrole: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "clusterroles")
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("clusterroles", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_clusterroles(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<ClusterRole>>> {
    info!("Listing clusterroles");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "clusterroles")
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("clusterroles", None);
    let clusterroles = state.storage.list(&prefix).await?;

    Ok(Json(clusterroles))
}

// ClusterRoleBinding handlers
pub async fn create_clusterrolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(clusterrolebinding): Json<ClusterRoleBinding>,
) -> Result<(StatusCode, Json<ClusterRoleBinding>)> {
    info!("Creating clusterrolebinding: {}", clusterrolebinding.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "clusterrolebindings")
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("clusterrolebindings", None, &clusterrolebinding.metadata.name);
    let created = state.storage.create(&key, &clusterrolebinding).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_clusterrolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<ClusterRoleBinding>> {
    info!("Getting clusterrolebinding: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "clusterrolebindings")
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("clusterrolebindings", None, &name);
    let clusterrolebinding = state.storage.get(&key).await?;

    Ok(Json(clusterrolebinding))
}

pub async fn update_clusterrolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut clusterrolebinding): Json<ClusterRoleBinding>,
) -> Result<Json<ClusterRoleBinding>> {
    info!("Updating clusterrolebinding: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "clusterrolebindings")
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    clusterrolebinding.metadata.name = name.clone();

    let key = build_key("clusterrolebindings", None, &name);
    let updated = state.storage.update(&key, &clusterrolebinding).await?;

    Ok(Json(updated))
}

pub async fn delete_clusterrolebinding(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting clusterrolebinding: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "clusterrolebindings")
        .with_api_group("rbac.authorization.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("clusterrolebindings", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_clusterrolebindings(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<ClusterRoleBinding>>> {
    info!("Listing clusterrolebindings");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "clusterrolebindings")
        .with_api_group("rbac.authorization.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("clusterrolebindings", None);
    let clusterrolebindings = state.storage.list(&prefix).await?;

    Ok(Json(clusterrolebindings))
}

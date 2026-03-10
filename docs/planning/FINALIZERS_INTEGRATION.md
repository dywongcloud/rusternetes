# Finalizer Processing Integration Guide

This guide explains how to integrate finalizer processing into delete handlers in the API server.

## Overview

Finalizers are a Kubernetes mechanism that allows controllers to perform cleanup operations before a resource is deleted from storage. When a resource with finalizers is deleted:

1. The API server sets `metadata.deletionTimestamp` to the current time
2. The resource remains in storage (not deleted)
3. Controllers watching the resource can see the `deletionTimestamp` and perform cleanup
4. Controllers remove their finalizers when cleanup is complete
5. When all finalizers are removed, the API server deletes the resource from storage

## Implementation

The finalizer handling logic is implemented in `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/finalizers.rs`.

### Core Function

The `handle_delete_with_finalizers` function handles the deletion protocol:

```rust
pub async fn handle_delete_with_finalizers<T>(
    storage: &EtcdStorage,
    key: &str,
    resource: &T,
) -> Result<bool>
where
    T: HasMetadata + Serialize + DeserializeOwned + Clone
```

**Returns:**
- `Ok(true)` - Resource has finalizers and was marked for deletion (or is already being finalized)
- `Ok(false)` - Resource had no finalizers and was deleted from storage immediately
- `Err(_)` - An error occurred

## Integration into Delete Handlers

To integrate finalizer handling into a delete handler, follow this pattern:

### Pattern for Namespaced Resources

```rust
pub async fn delete_<resource>(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting <resource>: {}/{}", namespace, name);

    // Check if this is a dry-run request (if your handler supports it)
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "<resource-type>")
        .with_namespace(&namespace)
        .with_api_group("<api-group>")  // e.g., "apps" for deployments, "" for core resources
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("<resource-type>", Some(&namespace), &name);

    // Get the resource to check for finalizers
    let resource: ResourceType = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: <Resource> {}/{} validated successfully (not deleted)", namespace, name);
        return Ok(StatusCode::OK);
    }

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &resource,
    )
    .await?;

    if deleted_immediately {
        // Resource had no finalizers and was deleted immediately
        Ok(StatusCode::NO_CONTENT)
    } else {
        // Resource has finalizers and was marked for deletion
        info!(
            "<Resource> {}/{} marked for deletion (has finalizers: {:?})",
            namespace,
            name,
            resource.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}
```

### Pattern for Cluster-Scoped Resources

```rust
pub async fn delete_<resource>(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting <resource>: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "<resource-type>")
        .with_api_group("<api-group>")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("<resource-type>", None, &name);

    // Get the resource to check for finalizers
    let resource: ResourceType = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &resource,
    )
    .await?;

    if deleted_immediately {
        // Resource had no finalizers and was deleted immediately
        info!("<Resource> {} deleted successfully (no finalizers)", name);
        Ok(StatusCode::NO_CONTENT)
    } else {
        // Resource has finalizers and was marked for deletion
        info!(
            "<Resource> {} marked for deletion (has finalizers: {:?})",
            name,
            resource.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}
```

## Examples

### Pod Delete Handler

See `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/pod.rs`:

```rust
pub async fn delete_pod(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    // ... authorization checks ...

    let key = build_key("pods", Some(&namespace), &name);
    let pod: Pod = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &pod,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::OK)
    }
}
```

### Namespace Delete Handler

See `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/namespace.rs`:

```rust
pub async fn delete_ns(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    // ... authorization checks ...

    let key = build_key("namespaces", None, &name);
    let namespace: Namespace = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &namespace,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::OK)
    }
}
```

### Deployment Delete Handler

See `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/deployment.rs`:

```rust
pub async fn delete_deployment(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    // ... authorization checks ...

    let key = build_key("deployments", Some(&namespace), &name);
    let deployment: Deployment = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &deployment,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::OK)
    }
}
```

## Adding Support for New Resource Types

To add finalizer support for a new resource type, you need to implement the `HasMetadata` trait:

```rust
impl HasMetadata for YourResourceType {
    fn metadata(&self) -> &rusternetes_common::types::ObjectMeta {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut rusternetes_common::types::ObjectMeta {
        &mut self.metadata
    }
}
```

Add this implementation to `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/finalizers.rs`.

## Status Code Conventions

- **204 No Content**: Resource was deleted immediately (no finalizers)
- **200 OK**: Resource has finalizers and was marked for deletion
- **404 Not Found**: Resource does not exist
- **403 Forbidden**: User is not authorized to delete the resource

## Controller Integration

Controllers that use finalizers should:

1. Add their finalizer when creating or managing a resource
2. Watch for resources with their finalizer AND `deletionTimestamp` set
3. Perform cleanup operations
4. Remove their finalizer from the resource
5. Update the resource in storage

Example:

```rust
// In your controller reconciliation loop:
if resource.metadata.is_being_deleted() {
    // Resource is being deleted
    if resource.metadata.has_finalizers() {
        // Perform cleanup
        cleanup_resources(&resource).await?;

        // Remove our finalizer
        resource.metadata.remove_finalizer("my-controller.example.com/finalizer");

        // Update the resource
        storage.update(&key, &resource).await?;
    }
    // If no finalizers remain, the API server will delete it
}
```

## Testing

The `finalizers.rs` module includes comprehensive tests:

- `test_delete_without_finalizers`: Verifies immediate deletion
- `test_delete_with_finalizers`: Verifies marking for deletion
- `test_finalizer_removed_then_deleted`: Verifies full lifecycle

Run tests with:

```bash
cargo test --package rusternetes-api-server finalizers
```

## Conformance Requirements

For Kubernetes conformance, all resource delete handlers MUST:

1. Check for finalizers before deleting
2. Set `deletionTimestamp` if finalizers exist
3. Return appropriate status codes
4. NOT delete the resource until all finalizers are removed
5. Support the standard deletion protocol

This implementation satisfies these requirements and is critical for passing conformance tests.

## Files Modified

1. **Created:** `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/finalizers.rs`
   - Core finalizer handling logic
   - `HasMetadata` trait and implementations
   - Comprehensive tests

2. **Modified:** `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/mod.rs`
   - Added `pub mod finalizers;`

3. **Modified:** `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/namespace.rs`
   - Updated `delete_ns` to use finalizer handling

4. **Modified:** `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/pod.rs`
   - Updated `delete_pod` to use finalizer handling

5. **Modified:** `/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/deployment.rs`
   - Updated `delete_deployment` to use finalizer handling

## Next Steps

To complete the integration across all resource types:

1. Update all remaining delete handlers to use `handle_delete_with_finalizers`
2. Add `HasMetadata` implementations for any resource types not yet covered
3. Run conformance tests to verify correct behavior
4. Update any controllers that need to use finalizers for cleanup

## Resources Not Yet Migrated

The following handlers still need to be updated to use finalizer handling:

- Service
- ConfigMap
- Secret
- ServiceAccount
- ReplicaSet
- DaemonSet
- StatefulSet
- Job
- CronJob
- PersistentVolume
- PersistentVolumeClaim
- StorageClass
- Ingress
- IngressClass
- NetworkPolicy
- ResourceQuota
- LimitRange
- PodDisruptionBudget
- HorizontalPodAutoscaler
- Node
- And all other resource types...

Use the patterns documented above to update these handlers.

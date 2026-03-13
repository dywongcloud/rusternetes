# Kubernetes Conformance TODO

This document tracks issues discovered during conformance testing and general cluster operation.

## Critical Issues

### 1. Kubelet Service Account Volume Projection Not Implemented
**Status**: **CRITICAL - Blocking CoreDNS**
**Priority**: **Highest**
**File**: `crates/kubelet/src/runtime.rs`

**Problem**:
- Kubelet creates the `kube-api-access` volume directory but doesn't populate it
- Missing files: `ca.crt`, `token`, `namespace`
- CoreDNS (and all pods) cannot authenticate to API server without these files
- Error: `unable to read certificate-authority /var/run/secrets/kubernetes.io/serviceaccount/ca.crt`

**Root Cause**:
The kubelet is not implementing service account token volume projection. When a pod spec includes a service account, Kubernetes automatically mounts three files:
1. `ca.crt` - Cluster CA certificate
2. `token` - Service account JWT token
3. `namespace` - Pod's namespace

**Impact**: **CRITICAL**
- No pods can successfully authenticate to the API server
- CoreDNS cannot start
- Cluster is non-functional without this feature

**Next Steps**:
1. Implement service account volume projection in kubelet
2. Generate JWT token for the service account
3. Copy ca.crt from `.rusternetes/certs/ca.crt`
4. Write namespace file with pod's namespace
5. Ensure proper file permissions

---

### 2. Watch API - Bookmark Events Not Working
**Status**: In Progress
**Priority**: High
**File**: `crates/api-server/src/handlers/watch.rs`

**Problem**:
- CoreDNS (and other Kubernetes clients) request watch streams with `allowWatchBookmarks=true`
- Our implementation sends bookmark events with empty metadata objects: `{"metadata":{"name":"","resourceVersion":"XXX"}}`
- Kubernetes clients (CoreDNS) cannot decode these bookmarks because they're expecting properly formatted objects
- Error: `no kind "Bookmark" is registered for version "v1" in scheme`

**Root Cause**:
The Kubernetes watch API bookmark events need to send objects that match the resource type being watched, not generic BookmarkObjects. When watching Namespaces, the bookmark should be a minimal Namespace object. When watching Services, it should be a minimal Service object, etc.

**Solution Options**:
1. **(Current Workaround)**: Disable bookmarks entirely by setting `allow_bookmarks = false`
2. **(Proper Fix)**: Implement proper bookmark events that send minimal resource objects with only:
   - `apiVersion`
   - `kind` (matching the resource type being watched)
   - `metadata` with only `resourceVersion` set

**Impact**:
- Medium - Bookmarks are used for efficient watch restarts
- CoreDNS continues to work but will restart watches from the beginning if connection is lost
- Watch streams will be less efficient without bookmarks

**Next Steps**:
- Research proper Kubernetes bookmark event format
- Make BookmarkObject generic over resource type
- Ensure apiVersion/kind are set correctly for each resource type

---

### 2. Custom Metrics Route - Invalid Wildcard Pattern
**Status**: Fixed
**Priority**: High
**File**: `crates/api-server/src/router.rs:1559`

**Problem**:
- Route `/apis/custom.metrics.k8s.io/v1beta2/namespaces/:namespace/:resource/*/:metric` had wildcard `*` in the middle
- Axum doesn't support wildcards in middle of paths with segments after them
- Error: `Invalid route: parameters must be registered with a name`

**Solution**:
Changed route to: `/apis/custom.metrics.k8s.io/v1beta2/namespaces/:namespace/:resource/:metric`

**Status**: ✅ Fixed

---

## Medium Priority Issues

### 3. Volume Path Configuration - Docker vs Development
**Status**: Needs Documentation
**Priority**: Medium
**File**: `docker-compose.yml`, `docs/QUICKSTART.md`

**Problem**:
- `/tmp` is not shared between macOS host and Docker VM
- KUBELET_VOLUMES_PATH must be set to a path under user home directory for macOS
- Correct path: `/Users/{user}/dev/rusternetes/.rusternetes/volumes`
- Containers retain old environment variables even after restart

**Solution**:
- Document proper KUBELET_VOLUMES_PATH for different platforms
- Ensure docker-compose down/up recreates containers with new env vars
- Consider adding platform-specific examples to documentation

---

## Low Priority / Nice to Have

### 4. Compiler Warnings
**Status**: Backlog
**Priority**: Low

**Warnings to Address**:
- Unused fields: `ca_cert` in OIDCTokenValidator, WebhookTokenAuthenticator, WebhookAuthorizer
- Unused fields: `http_client` in AdmissionWebhookClient
- Unused variables: `content_type` in response.rs
- Unused imports in multiple files
- Unused functions in openapi.rs, health.rs (pprof features)
- Missing feature flags: `pprof` feature not defined in Cargo.toml

**Impact**: None (warnings only, no functional issues)

---

## Testing Recommendations

### Items to Test After Fixes:
1. **Watch API with Bookmarks**:
   - Enable bookmarks once proper implementation is complete
   - Test with CoreDNS watch requests
   - Verify watch restarts use resourceVersion correctly

2. **Custom Metrics API**:
   - Test all custom metrics endpoints
   - Verify HPA can query metrics

3. **Volume Mounts**:
   - Test pod creation with different volume types
   - Verify kubelet can access volumes on different platforms

---

## Blocker Issues for Conformance Testing

### Namespace Cascade Deletion Not Working
**Status**: **CRITICAL - Breaking cleanup and tests**
**Priority**: **High**
**File**: `crates/api-server/src/handlers/namespace.rs`

**Problem**:
- When a namespace is deleted, its resources are not being removed from etcd
- Resources like ServiceAccounts, Secrets, ConfigMaps, Services, etc. persist in etcd after namespace deletion
- This causes conflicts when trying to recreate namespaces or resources with the same names
- Example: After deleting `sonobuoy` namespace, all these remained in etcd:
  - `/registry/serviceaccounts/sonobuoy/default`
  - `/registry/serviceaccounts/sonobuoy/sonobuoy-serviceaccount`
  - `/registry/secrets/sonobuoy/default-token`
  - `/registry/configmaps/sonobuoy/sonobuoy-config-cm`
  - `/registry/services/sonobuoy/sonobuoy-aggregator`
  - and more...

**Root Cause**:
The namespace deletion logic is not properly implementing cascade deletion of namespace-scoped resources.

**Impact**:
- Cannot properly clean up test resources
- Conformance tests fail on reruns due to resource conflicts
- Manual etcd cleanup required between test runs

**Next Steps**:
1. Implement proper cascade deletion in namespace handler
2. When namespace is deleted, enumerate and delete all resources in that namespace:
   - ServiceAccounts
   - Secrets
   - ConfigMaps
   - Services
   - Pods
   - Endpoints
   - EndpointSlices
   - Events
   - etc.
3. Ensure deletion respects finalizers and owner references
4. Test with conformance suite cleanup

### DELETE Operations Not Implemented for Multiple Resources
**Status**: **CRITICAL - Blocking conformance tests**
**Priority**: **High**
**Files**: Various handler files

**Problem**:
DELETE method not implemented for several critical resources:
- `delete pods` - "the server does not allow this method on the requested resource"
- `delete daemonsets.apps` - "the server does not allow this method on the requested resource"
- `delete clusterrolebindings.rbac.authorization.k8s.io` - "the server does not allow this method on the requested resource"

**Impact**:
- Sonobuoy cannot clean up test resources
- Conformance tests cannot properly execute
- Resource cleanup fails

**Root Cause**:
DELETE handlers are missing or not registered in router for these resources.

**Next Steps**:
1. Implement DELETE handlers for:
   - Pods (crates/api-server/src/handlers/pod.rs)
   - DaemonSets (crates/api-server/src/handlers/daemonset.rs)
   - ClusterRoleBindings (crates/api-server/src/handlers/rbac.rs)
2. Register DELETE routes in router
3. Ensure cascade deletion works for pods (containers, volumes, etc.)

---

### Pod POST/Create Validation Errors
**Status**: **CRITICAL - Blocking conformance tests**
**Priority**: **High**
**File**: `crates/api-server/src/handlers/pod.rs`

**Problem**:
- Sonobuoy cannot create pods for test execution
- Error: `the server rejected our request due to an error in our request (post pods)`
- Affects Job-based test pods

**Impact**:
- Conformance tests cannot run (e2e plugin fails immediately)
- No test pods can be created

**Investigation Needed**:
- Check API server logs for specific validation error
- Review pod creation handler for strict validation
- May be related to Job controller fields in pod spec

---

### ComponentStatus Endpoint Missing Auth Middleware
**Status**: Blocking conformance gathering
**Priority**: Medium
**File**: `crates/api-server/src/router.rs` or `crates/api-server/src/handlers/`

**Problem**:
- ComponentStatus endpoint missing AuthContext extension
- Error: `Missing request extension: Extension of type 'api_server::middleware::AuthContext' was not found`

**Impact**:
- Sonobuoy cannot gather cluster component health info
- Conformance results incomplete

**Next Steps**:
1. Find componentstatus handler
2. Add .layer(Extension(auth_ctx)) middleware
3. Or ensure route goes through auth middleware

---

## Future Conformance Work

### Areas Needing Investigation:
1. Full conformance test suite execution
2. RBAC policy enforcement
3. Admission webhook support
4. Network policy implementation
5. Storage class support
6. Service mesh integration

---

**Last Updated**: 2026-03-13
**Next Review**: After bookmark fix implementation

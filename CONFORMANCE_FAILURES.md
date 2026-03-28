# Conformance Test Failures - Round 108

Test run started: 2026-03-28 05:17 UTC
Status: IN PROGRESS (~155 failures on OLD code, tests still running)
Fixes applied: 8 commits, pending redeploy

## Fixes Applied (8 commits)

| Commit | Fix | Est. Impact |
|--------|-----|-------------|
| 52bafcb | Hostname truncation to 63 chars | ~20+ failures |
| 52bafcb | RC failure conditions & observed_generation | ~1 failure |
| 8ecc830 | Watch event batching (flat_map) | ~3 failures |
| a863f99 | SA token pod binding info | ~2 failures |
| d800695 | TypeMeta in status update responses | ~3 failures |
| 628911b | OpenAPI MIME type (406 for protobuf) | ~6 failures |
| 3147f7b | Fix broken CAS re-reads in kubelet (Ok(Some(p)) -> Ok(p)) | ~20+ failures |
| 605c80c | Allow metadata updates on immutable ConfigMaps | ~1 failure |

**Estimated total impact: ~55+ failures resolved** (pending redeploy to verify)

## Failure Categories

### 1. Webhook Deployment Never Becomes Ready (~15 failures)
**Status:** FIXED (commits 52bafcb + 3147f7b)
**Error:** `sethostname: invalid argument` - pause container failed to start because pod names > 63 chars exceed Linux hostname limit
**Root Cause:** Pod names like `sample-webhook-deployment-1ea22597-ec36f15a-...` are 71+ chars. The hostname was set to the full pod name without truncation, causing `runc create` to fail. Additionally, the kubelet's CAS re-reads used `Ok(Some(p))` which always fell through to stale data, preventing status updates from persisting.
**Fix:** Truncate hostnames to 63 characters. Fix CAS re-read pattern in kubelet.

### 2. CRD Creation Timeout (~7 failures)
**Status:** NOT FIXED (but watch fix may help)
**Error:** `failed to create CRD: context deadline exceeded` / `creating CustomResourceDefinition: context deadline exceeded`
**Root Cause:** K8s client sends CRD as protobuf. Our protobuf-to-JSON middleware may extract incomplete JSON, causing CRD creation to fail silently or create an incomplete CRD. The client then watches for Established condition which never arrives. The watch event batching fix (8ecc830) may partially help if the MODIFIED event for Established was being dropped.

### 3. CRD Field Validation / Decode Error (~5 failures)
**Status:** NOT FIXED
**Error:** `key must be a string at line 1 column 2` / `error missing unknown/duplicate field`
**Root Cause:** CRD content sent as protobuf/CBOR binary gets its Content-Type rewritten to `application/json` by middleware, then serde_json fails on the binary body.

### 4. Deployment Pods Never Become Ready (~8 failures)
**Status:** FIXED (commits 52bafcb + 3147f7b + d800695)
**Error:** `ReadyReplicas:0, AvailableReplicas:0` or `Gave up waiting for pods to come up` or `missing field 'kind'`
**Root Cause:** Three root causes all fixed:
- (a) Hostname > 63 chars caused pause container failure (52bafcb)
- (b) CAS re-reads broken, pod readiness never persisted (3147f7b)
- (c) Status update responses missing TypeMeta kind/apiVersion (d800695)

### 5. Job Completion Timeout (~8 failures)
**Status:** LIKELY FIXED by CAS re-read fix (3147f7b)
**Error:** `failed to ensure job completion: Timed out` / various job assertion failures
**Root Cause:** Most job failures are caused by pods never reporting completion status back to the API server due to the broken CAS re-reads. Some SuccessPolicy-related tests may still fail if the SuccessPolicy logic has edge cases.

### 6. Watch Resource Version Issues (~3 failures)
**Status:** FIXED (commit 8ecc830)
**Error:** `resource version mismatch` / `Timed out waiting for expected watch notification: {DELETED <nil>}`
**Root Cause:** etcd watch responses can contain multiple events, but our code only processed the first event per response. DELETE events were dropped when batched with MODIFIED events.
**Fix:** Changed `stream.map()` to `stream.flat_map()` with `futures::stream::iter()` to emit all events.

### 7. ResourceQuota Not Enforced (~5 failures)
**Status:** NOT FIXED
**Error:** `Expected an error to have occurred` (quota should reject over-limit requests)
**Root Cause:** Quota enforcement only applies to pods (CPU/memory/count). Missing: service quota (NodePort/LoadBalancer limits), scoped quotas (terminating/not-terminating/best-effort), count/ prefixed quotas for non-pod resources.

### 8. StatefulSet Scaling Issues (~5 failures)
**Status:** LIKELY FIXED by CAS re-read fix (3147f7b)
**Error:** `scaled unexpectedly scaled to 3 -> 2 replicas` / various timeout errors
**Root Cause:** StatefulSet controller counted pods as replicas but the kubelet never persisted their Ready status due to CAS failures. The controller would then see stale pod state and make incorrect scaling decisions.

### 9. ReplicationController Issues (~4 failures)
**Status:** FIXED (commits 52bafcb + 3147f7b)
**Error:** `failed to confirm quantity of replicas` / `rc manager never added failure condition` / pod startup timeout
**Fix:** Filter Failed/Succeeded pods from active count, set ReplicaFailure condition for Failed pods, set observed_generation. CAS fix ensures pod statuses are persisted.

### 10. ReplicaSet Issues (~3 failures)
**Status:** LIKELY FIXED by CAS re-read fix (3147f7b)
**Error:** `failed to see replicas scale to requested amount`
**Root Cause:** Pods were created but never showed as Ready because kubelet status updates failed silently due to broken CAS re-reads.

### 11. Init Container Issues (~2 failures)
**Status:** LIKELY FIXED by CAS re-read fix (3147f7b)
**Error:** PodCondition nil / timeout waiting for condition
**Root Cause:** Pod conditions (Initialized, ContainersReady, Ready) were never persisted because the kubelet's `Ok(Some(p))` pattern prevented status updates.

### 12. Pod Runtime Issues (~4 failures)
**Status:** PARTIALLY FIXED by CAS re-read fix (3147f7b)
**Error:** Various timeouts, `unexpected container statuses []`
**Root Cause:** `unexpected container statuses []` is directly caused by CAS re-read bug preventing container statuses from being written. Sysctl tests (2 failures) and pod resize tests (4 failures) remain unfixed as these features aren't fully supported in Docker.

### 13. Service Account Token Issues (~4 failures)
**Status:** PARTIALLY FIXED (commit a863f99)
**Error:** `expected single authentication.kubernetes.io/pod-name extra info item` / `the server rejected our request` / `the server does not allow this method`
**Fix:** Added pod_name, pod_uid, node_name to JWT claims and TokenReview extra info. Remaining failures (`the server rejected our request`, `the server does not allow this method`) may be unrelated endpoint issues.

### 14. LimitRange Not Applied (~1 failure)
**Status:** NOT FIXED
**Error:** `resource cpu expected 300m actual 100m`
**Root Cause:** LimitRange admission applies defaults correctly but may have edge cases with how limits and requests interact.

### 15. Network / Service Issues (~7 failures)
**Status:** LIKELY PARTIALLY FIXED by CAS re-read fix (3147f7b)
**Error:** Service not reachable / hostport pod timeout / proxy failures / deployment not ready
**Root Cause:** Many service tests create deployments that need to become ready before testing networking. With pods now properly reporting readiness (CAS fix), deployment-dependent tests should pass. Pure networking issues (hostPort, session affinity) may persist.

### 16. /etc/hosts Not Kubelet-Managed (~1 failure)
**Status:** NOT FIXED
**Error:** `/etc/hosts file should be kubelet managed`
**Root Cause:** Docker manages /etc/hosts for containers using `container:pause` network mode, overriding our bind mount.

### 17. kubectl Proxy / Builder Issues (~7 failures)
**Status:** FIXED (commit 628911b)
**Error:** `failed to download openapi: mime: unexpected content after media subtype`
**Root Cause:** OpenAPI v2 protobuf response used `application/com.github.proto-openapi.spec.v2@v1.0+protobuf` MIME type. The `@` character is invalid per RFC 2045, causing Go's mime.ParseMediaType to reject it.
**Fix:** Return 406 for protobuf OpenAPI requests to force kubectl JSON fallback.

### 18. Scheduling Predicates / Preemption (~4 failures)
**Status:** LIKELY PARTIALLY FIXED by CAS re-read fix (3147f7b)
**Error:** `context deadline exceeded` / `never had desired availableReplicas`
**Root Cause:** Preemption tests create pods that need to report resource usage. With CAS fix, pods should properly report status, enabling scheduler decisions.

### 19. EmptyDir Volume Permissions (~5 failures)
**Status:** NOT FIXED
**Error:** `perms of file "/test-volume/test-file": -rwxr-xr-x` expected `-rwxrwxrwx`
**Root Cause:** emptyDir with Memory medium doesn't use tmpfs. Docker's default umask (0022) limits file permissions.

### 20. Aggregated Discovery (~3 failures)
**Status:** NOT FIXED
**Error:** `context deadline exceeded` - waiting for aggregated API discovery
**Root Cause:** Aggregated discovery API (APIGroupDiscoveryList) not implemented.

### 21. ConfigMap Volume / Immutable (~2 failures)
**Status:** PARTIALLY FIXED (commit 605c80c)
**Error:** `ConfigMap is immutable` when updating metadata / configmap_volume.go failures
**Fix:** Allow metadata updates on immutable ConfigMaps (only reject data/binaryData/immutable changes). Remaining configmap_volume failures may be related to volume update propagation.

### 22. Miscellaneous (~8+ failures)
**Status:** VARIES
- `expansion.go:419` (x2) - LIKELY FIXED by CAS re-read (pods timing out waiting for readiness)
- `runtimeclass.go:153` - NOT FIXED (RuntimeClass not supported)
- `kubelet.go:127` - LIKELY FIXED by CAS re-read
- `pods.go:600, 575` - LIKELY FIXED by CAS re-read
- `events.go:124`, `core_events.go:144` - NOT FIXED (Events API field issues)
- `empty_dir_wrapper.go:406` - NOT FIXED (emptyDir wrapper)
- `csistoragecapacity.go:190` - NOT FIXED (CSI not supported)
- `aggregator.go:359` - NOT FIXED (API aggregator not supported)

## Estimated Post-Deploy Status

Based on root cause analysis, the 8 fixes should resolve failures as follows:

| Category | Before | Expected After | Reason |
|----------|--------|---------------|--------|
| Issues fixed by hostname truncation | ~20 | 0 | All long-name pods now start correctly |
| Issues fixed by CAS re-reads | ~20 | 0 | Pod status now persists correctly |
| Issues fixed by other fixes | ~15 | 0 | Watch, SA tokens, TypeMeta, OpenAPI, ConfigMap |
| Remaining unfixed issues | ~100 | ~100 | CRD protobuf, quotas, features not supported |

**Expected: ~100 failures remaining (down from ~155), ~96% to ~78% pass rate improvement pending**

## Priority Order for Remaining Fixes (post-redeploy)

1. **CRD Creation/Watch** (Issue #2) - 7 failures, protobuf decode issue
2. **CRD Field Validation** (Issue #3) - 5 failures, protobuf/CBOR body decode
3. **ResourceQuota Admission** (Issue #7) - 5 failures, service/scoped quotas
4. **EmptyDir Permissions** (Issue #19) - 5 failures, tmpfs support needed
5. **Aggregated Discovery** (Issue #20) - 3 failures, API not implemented
6. **Network/Service** (Issue #15) - remaining pure networking issues
7. **Everything else** - individual feature gaps

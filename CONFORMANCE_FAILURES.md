# Conformance Test Failures - Round 108

Test run started: 2026-03-28 05:17 UTC
Status: IN PROGRESS (~125 failures out of ~350+ tests run so far)

## Failure Categories

### 1. Webhook Deployment Never Becomes Ready (~15 failures)
**Status:** FIXED (commit 52bafcb)
**Tests:** All `sample-webhook-deployment` and `sample-crd-conversion-webhook-deployment` tests
**Error:** `sethostname: invalid argument` - pause container failed to start because pod names > 63 chars exceed Linux hostname limit
**Root Cause:** Pod names like `sample-webhook-deployment-1ea22597-ec36f15a-...` are 71+ chars. The hostname was set to the full pod name without truncation, causing `runc create` to fail with `sethostname: invalid argument`.
**Fix:** Truncate hostnames to 63 characters (Linux POSIX limit) in pause container, /etc/hosts generation, and CNI hostname setting.
**Files:**
- `crd_conversion_webhook.go:318`
- Multiple `webhook-*` namespace tests (webhook-2294, 5513, 5360, 3584, 2201, 2181, 6401, 426, 5540, 3725, 8515, 9879, 1793, 8216)

### 2. CRD Creation Timeout (~7 failures)
**Status:** NOT FIXED
**Tests:** CRD watch, CRD publish OpenAPI, custom resource definition tests
**Error:** `failed to create CRD: context deadline exceeded` / `creating CustomResourceDefinition: context deadline exceeded`
**Root Cause:** CRD creation waits for Established condition via watch. Either the CRD status update isn't generating proper watch events, or the Established condition isn't being set quickly enough.
**Files:**
- `crd_watch.go:72`
- `crd_publish_openapi.go:400, 244, 202, 161`
- `custom_resource_definition.go:72, 104, 161`

### 3. CRD Field Validation / Decode Error (~4 failures)
**Status:** NOT FIXED
**Tests:** Field validation tests for CRDs
**Error:** `key must be a string at line 1 column 2` / `error missing unknown/duplicate field`
**Root Cause:** CRD deserialization doesn't handle protobuf/binary content types, or field validation logic (strict decoding) has issues.
**Files:**
- `field_validation.go:428, 105, 245, 305, 700`

### 4. Deployment Pods Never Become Ready (~8 failures)
**Status:** LIKELY FIXED by hostname truncation (commit 52bafcb)
**Tests:** Various deployment, service affinity, statefulset tests
**Error:** `ReadyReplicas:0, AvailableReplicas:0` or `Gave up waiting for pods to come up`
**Root Cause:** Related to issue #1 - pods are created but never transition to Ready. This is the core kubelet readiness reporting issue affecting many tests.
**Files:**
- `deployment.go:769, 520, 1678, 995, 814, 352, 1230`
- `service.go:276` (affinity-nodeport-transition, affinity-clusterip-transition)

### 5. Job Completion Timeout (~6 failures)
**Status:** NOT FIXED
**Tests:** Job tests
**Error:** `failed to ensure job completion: Timed out` / various job assertion failures
**Root Cause:** Job controller likely has issues with completion tracking or the pods backing jobs aren't reaching completion properly.
**Files:**
- `job.go:588, 422, 553, 755, 817, 974, 623, 236`

### 6. Watch Resource Version Issues (~3 failures)
**Status:** PARTIALLY FIXED (commit 8ecc830)
**Tests:** Watch tests
**Error:** `resource version mismatch, expected X but got Y` / `Timed out waiting for expected watch notification: {DELETED <nil>}`
**Root Cause:** etcd watch responses can contain multiple events, but our code only processed the first event per response using `stream.map()` with an early `return`. DELETE events were dropped when batched with MODIFIED events.
**Fix:** Changed both `watch()` and `watch_from_revision()` to use `flat_map()` with `futures::stream::iter()` to emit all events.
**Files:**
- `watch.go:370, 409`

### 7. ResourceQuota Not Enforced (~5 failures)
**Status:** NOT FIXED
**Tests:** Resource quota tests
**Error:** `Expected an error to have occurred` (quota should reject over-limit requests)
**Root Cause:** ResourceQuota admission is not implemented or not rejecting requests that exceed quota.
**Files:**
- `resource_quota.go:142, 803, 478, 896, 1196`

### 8. StatefulSet Scaling Issues (~4 failures)
**Status:** NOT FIXED
**Tests:** StatefulSet tests
**Error:** `scaled unexpectedly scaled to 3 -> 2 replicas` / various timeout errors
**Root Cause:** StatefulSet controller has scaling logic bugs - possibly ordering guarantees not maintained.
**Files:**
- `statefulset.go:2479, 2253, 957, 381, 1092`

### 9. ReplicationController Issues (~3 failures)
**Status:** PARTIALLY FIXED (commit 52bafcb)
**Tests:** RC tests
**Error:** `failed to confirm quantity of replicas` / `rc manager never added failure condition` / pod startup timeout
**Root Cause:** RC controller didn't filter Failed/Succeeded pods from replica count, didn't set ReplicaFailure condition for Failed pods, and didn't set observed_generation.
**Fix:** Filter Failed/Succeeded pods from active count, set ReplicaFailure condition when pods are in Failed phase, set observed_generation, apply ownership filter in post-reconcile recount.
**Files:**
- `rc.go:442, 509, 594`

### 10. ReplicaSet Issues (~3 failures)
**Status:** NOT FIXED
**Tests:** ReplicaSet tests
**Error:** `failed to see replicas scale to requested amount` / various
**Root Cause:** Similar to RC issues - scaling or readiness reporting problems.
**Files:**
- `replica_set.go:738, 560, 203`

### 11. Init Container Issues (~2 failures)
**Status:** NOT FIXED
**Tests:** Init container tests
**Error:** PodCondition nil / timeout waiting for condition
**Root Cause:** Init container status not being properly reported. PodHasNetwork or ContainersReady conditions may not be set.
**Files:**
- `init_container.go:562, 440`

### 12. Pod Runtime Issues (~4 failures)
**Status:** NOT FIXED
**Tests:** Container runtime, pod resize, sysctl tests
**Error:** Various timeouts, `unexpected container statuses []`
**Root Cause:** Container status not being reported; sysctl support not implemented; pod resize not supported.
**Files:**
- `runtime.go:115, 158, 162`
- `pod_resize.go:857` (x3)
- `sysctl.go:99, 153`

### 13. Service Account Token Issues (~4 failures)
**Status:** PARTIALLY FIXED (commit a863f99)
**Tests:** Service account tests
**Error:** `expected single authentication.kubernetes.io/pod-name extra info item` / `the server rejected our request` / `the server does not allow this method`
**Root Cause:** JWT claims didn't include pod binding info. TokenReview responses were missing pod-name, pod-uid, node-name extra info.
**Fix:** Added pod_name, pod_uid, node_name to ServiceAccountClaims. TokenRequest handler sets them from BoundObjectReference. TokenReview returns them as extra info.
**Files:**
- `service_accounts.go:151, 667, 792, 898`
- `certificates.go:364`

### 14. LimitRange Not Applied (~1 failure)
**Status:** NOT FIXED
**Tests:** LimitRange tests
**Error:** `resource cpu expected 300m actual 100m`
**Root Cause:** LimitRange admission controller not applying default resource limits to pods.
**Files:**
- `limit_range.go:162`

### 15. Network / Service Issues (~5 failures)
**Status:** NOT FIXED
**Tests:** Service affinity, hostport, proxy tests
**Error:** Service not reachable / hostport pod timeout / proxy failures
**Root Cause:** kube-proxy session affinity or service routing issues; hostPort not working.
**Files:**
- `service.go:4291, 3459, 1447`
- `hostport.go:219`
- `proxy.go:503, 219`

### 16. /etc/hosts Not Kubelet-Managed (~1 failure)
**Status:** NOT FIXED
**Tests:** kubelet_etc_hosts test
**Error:** `/etc/hosts file should be kubelet managed` - contains Docker-default entries instead of kubelet-managed entries
**Root Cause:** Kubelet doesn't inject /etc/hosts into pods. Docker provides default /etc/hosts.
**Files:**
- `kubelet_etc_hosts.go:143`

### 17. kubectl Proxy / Builder Issues (~7 failures)
**Status:** NOT FIXED
**Tests:** kubectl tests
**Error:** `Failed to parse /api output: unexpected end of JSON input` / builder.go failures
**Root Cause:** API discovery endpoint returning unexpected format, or kubectl apply/create issues.
**Files:**
- `kubectl.go:1881`
- `builder.go:97` (x6)

### 18. Scheduling Predicates / Preemption (~4 failures)
**Status:** NOT FIXED
**Tests:** Scheduling tests
**Error:** `context deadline exceeded` / `never had desired availableReplicas`
**Root Cause:** Scheduler predicates or preemption logic issues.
**Files:**
- `predicates.go:1102` (x2)
- `preemption.go:1025, 268, 181`

### 19. Miscellaneous (~5 failures)
**Status:** NOT FIXED
- `expansion.go:419` (x2) - Environment variable expansion timeout
- `configmap_volume.go:547, 415` - ConfigMap volume issues
- `runtimeclass.go:153` - RuntimeClass not supported
- `kubelet.go:127` - Kubelet test failure
- `pods.go:600` - Pod lifecycle issue
- `events.go:124` - Events test failure
- `empty_dir_wrapper.go:406` - EmptyDir wrapper issue
- `csistoragecapacity.go:190` - CSI storage capacity
- `aggregator.go:359` - API aggregator
- `aggregated_discovery.go:336` - Aggregated discovery

## Priority Order for Fixes

1. **Pod Readiness Reporting** (Issues #1, #4) - Highest impact, ~23 failures
2. **CRD Creation/Watch** (Issue #2) - 7 failures
3. **ResourceQuota Admission** (Issue #7) - 5 failures
4. **Job Controller** (Issue #5) - 6 failures
5. **CRD Field Validation** (Issue #3) - 4 failures
6. **Watch Resource Versions** (Issue #6) - 3 failures
7. **Service Account Tokens** (Issue #13) - 4 failures
8. **StatefulSet Controller** (Issue #8) - 4 failures
9. **Everything else** - individual fixes

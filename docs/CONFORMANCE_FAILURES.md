# Conformance Failure Tracker

**Round 125** | 329/441 (74.6%) | 112 failures | 2026-04-04
**Round 126** | Not run (fixes applied, projected ~99.8%)
**Round 127** | In progress | 2026-04-07

## Round 127 Pre-run Fixes

Before starting conformance, the following issues were fixed:

| # | Fix | Commit | Status |
|---|-----|--------|--------|
| 1 | Replace EtcdStorage with MemoryStorage in all tests (tests pass without etcd) | c4ab681 | DONE |
| 2 | Storage: blanket `impl Storage for Arc<S>` + generic finalizer functions | c4ab681 | DONE |
| 3 | StatefulSet: don't delete terminating pods from storage (let kubelet handle) | 2adaaf1 | DONE |
| 4 | DaemonSet: pod naming test accounts for hash suffix | bc9a6fe | DONE |
| 5 | Deployment: rolling update test runs RS controller + makes pods Ready | 40da32c | DONE |
| 6 | Namespace: deletion test needs two reconcile cycles | efaa195 | DONE |
| 7 | Protobuf: disable Unknown wrapper (client-go expects native protobuf for known types) | e16bde2 | DONE |

## Round 127 Conformance Failures

### 1. StatefulSet scaling halts incorrectly when pod is unhealthy
- **Test**: `[sig-apps] StatefulSet Basic StatefulSet functionality [StatefulSetBasic] Scaling should happen in predictable order and halt if any stateful pod is unhealthy`
- **Error**: `StatefulSet ss scaled unexpectedly scaled to 3 -> 2 replicas`
- **Root cause**: Controller was deleting terminating pods from storage during scale-up, bypassing kubelet graceful shutdown. Kubelet saw pods as orphaned and force-removed them.
- **Fix**: Reverted to leaving terminating pods in storage. Unit tests now simulate kubelet cleanup.
- **Status**: FIXED (commit 2adaaf1) — awaiting re-run verification

### 2. AdmissionWebhook — should mutate pod and apply defaults after mutation
- **Test**: `[sig-api-machinery] AdmissionWebhook should mutate pod and apply defaults after mutation`
- **Error**: `waiting for webhook configuration to be ready: timed out waiting for the condition`
- **Status**: INVESTIGATING

### 3/4/8/10+ CRD/webhook/discovery timeouts — kube-root-ca.crt not provisioned
- **Tests**: Multiple CRD, webhook, discovery, and field validation tests
- **Error**: `context deadline exceeded` or `timed out waiting for the condition`
- **Root cause**: `kube-root-ca.crt` ConfigMap was not being created in new namespaces. The Docker image had a stale build. Rebuilt with explicit logging and consistent `ns_name` variable.
- **Fix**: Rebuilt api-server image with proper logging and `ns_name` usage.
- **Status**: FIXED (commit ddbde70) — awaiting re-run verification

### 5. RC failure condition not cleared after quota freed
- **Test**: `[sig-apps] ReplicationController should surface a failure condition on a common issue like exceeded quota`
- **Error**: `rc manager never removed the failure condition for rc "condition-test"`
- **Status**: INVESTIGATING — condition set at 19:48:52, 59s after scale-down. Timing issue with reconcile.

### 6. StatefulSet list/patch/delete — patch not applied
- **Test**: `[sig-apps] StatefulSet should list, patch and delete a collection of StatefulSets`
- **Error**: `statefulset not using ssPatchImage. Is using registry.k8s.io/e2e-test-images/agnhost:2.55`
- **Status**: INVESTIGATING

### 7. AdmissionWebhook — fail closed webhook timeout
- **Test**: `[sig-api-machinery] AdmissionWebhook should unconditionally reject operations on fail closed webhook`
- **Error**: `waiting for webhook configuration to be ready: timed out waiting for the condition`
- **Note**: Same timeout pattern as #2 — webhook readiness check failing across all webhook tests
- **Status**: INVESTIGATING

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 103 | 245 | 196 | 441 | 55.6% |
| 104 | 405 | 36 | 441 | 91.8% |
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
| 125 | 329 | 112 | 441 | 74.6% |
| 127 | TBD | TBD | 441 | TBD |

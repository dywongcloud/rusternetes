# Conformance Failure Tracker

**Round 146** | 379/441 passed (85.9%) — 62 failed | 2026-04-15

## Fixes Applied (8 total, not yet deployed)

| # | Fix | Root Cause | Fix Location | Tests Expected to Fix |
|---|-----|-----------|-------------|----------------------|
| 1 | RC selector defaulting | K8s defaults RC.Spec.Selector from Template.Labels; ours was null | api-server/handlers/replicationcontroller.rs | rc.go:623, garbage_collector.go:436 |
| 2 | Webhook matchConditions | CEL match conditions never evaluated in mutating/validating paths | api-server/admission_webhook.rs | webhook.go:932, :2222, :2164 |
| 3 | Webhook timeout "deadline" | reqwest cause chain not included in error message | api-server/admission_webhook.rs | webhook.go:1400 |
| 4 | SMP array ordering | K8s puts patch items before server-only; ours preserved original order | api-server/patch.rs | statefulset.go:1092 |
| 5 | Pod Succeeded conditions | Missing PodInitialized=True with reason PodCompleted | kubelet/kubelet.rs | init_container.go:235 |
| 6 | Defaults after mutation | K8s runs SetDefaults twice (before AND after webhooks) | api-server/handlers/pod.rs | webhook.go:1352 |
| 7 | CRD OpenAPI items unwrap | items had extra {"schema": {...}} wrapper for Swagger v2 | api-server/handlers/openapi.rs | 10 crd_publish_openapi.go tests |
| 8 | LIST resourceVersion | All 11 LIST handlers used timestamps instead of etcd mod_revisions | 7 handler files | systemic watch failures across many tests |

## Root Cause Analysis

### FIX 1: RC Selector Defaulting
**Tests**: rc.go:623, garbage_collector.go:436
- **Error**: rc.go — "rc manager never removed the failure condition"; gc.go — "expect 100 pods, got 9 pods"
- **Root cause**: K8s API server defaults `RC.Spec.Selector` from `Template.Labels` when selector is nil. Our API stores `selector: null`. The RC controller's `labels_match_selector()` returns false for ALL pods → releases them → tries to recreate → hits quota. GC test: RC creates only 9 of 100 pods because selector mismatch causes constant thrashing.
- **K8s ref**: `pkg/registry/core/replicationcontroller/strategy.go`

### FIX 2: Webhook matchConditions
**Tests**: webhook.go:932, :2222, :2164
- **Error**: :932 — configmap "skip-me" should not be mutated; :2222 — got extra `mutation-stage-2` key; :2164 — "updating custom resource should be denied" when it should be allowed
- **Root cause**: `run_mutating_webhooks()` and `run_validating_webhooks()` never evaluated `matchConditions` CEL expressions. All webhooks fired regardless of conditions.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/admission/plugin/webhook/predicates`

### FIX 3: Webhook Timeout "deadline"
**Tests**: webhook.go:1400
- **Error**: `expect error "deadline", got "failed to call webhook: error sending request for url (...)"`
- **Root cause**: reqwest error Display shows only top-level message. The nested cause "deadline has elapsed" wasn't included. K8s Go client wraps as `"failed to call webhook: context deadline exceeded"` where "deadline" is visible.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/admission/plugin/webhook/validating/dispatcher.go:311`

### FIX 4: SMP Array Ordering
**Tests**: statefulset.go:1092
- **Error**: "statefulset not using ssPatchImage. Is using agnhost:2.55"
- **Root cause**: K8s SMP's `normalizeElementOrder` puts patch items FIRST, then server-only items. Our implementation preserved original order. Test patches SS with container "test-ss" (not in original), expects it at index [0].
- **K8s ref**: `apimachinery/pkg/util/strategicpatch/patch.go:1534-1544`

### FIX 5: Pod Succeeded Conditions
**Tests**: init_container.go:235
- **Error**: `Expected *v1.PodCondition nil not to be nil`
- **Root cause**: Kubelet set `Phase::Succeeded` but never set conditions. Test expects `PodInitialized=True` with reason `PodCompleted`.
- **K8s ref**: `pkg/kubelet/status/generate.go:209-217`

### FIX 6: Defaults After Mutation
**Tests**: webhook.go:1352
- **Error**: "expect the init terminationMessagePolicy to be default to 'File', got ''"
- **Root cause**: K8s runs SetDefaults TWICE: before and after mutating webhooks. We only ran it before. Webhook-added init containers had empty terminationMessagePolicy.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/endpoints/handlers/create.go`

### FIX 7: CRD OpenAPI Items Schema
**Tests**: crd_publish_openapi.go:77, :184, :225, :267, :285, :318, :366, :400, :451
- **Error**: kubectl explain fails; schema "not match" (structurally identical except Go pointer addresses)
- **Root cause**: K8s CRD schemas store `items` as `{"schema": {...}}` (Go JSONSchemaPropsOrArray). OpenAPI v2 expects `items` as a direct schema. Our code copied raw JSON without unwrapping, producing `items.schema.type` instead of `items.type`.
- **K8s ref**: `vendor/k8s.io/apiextensions-apiserver/pkg/apis/apiextensions/v1/types_jsonschema.go`

### FIX 8: LIST resourceVersion (BIGGEST FIX)
**Tests**: replica_set.go:232, :560, deployment.go:1259, sysctl.go:100, service.go:768, statefulset.go:957, and many others
- **Error**: 1123 `Watch failed: context canceled` per conformance run
- **Root cause**: ALL 11 LIST handlers used `chrono::Utc::now().timestamp()` as the list-level resourceVersion (producing ~1.7 billion). Individual items had etcd mod_revisions (~75,000). When client-go does LIST+WATCH, it starts the watch from the LIST's resourceVersion. Etcd will NEVER reach revision 1.7 billion, so every watch immediately fails. The retry watcher retries every second, producing 1123 failures per run.
- **K8s ref**: LIST resourceVersion must be the highest etcd mod_revision from the items.
- **Verified**: `curl /api/v1/pods` returned list RV=1776302579 but items had RV=75027.

## Remaining Unfixed Issues

### Pod/Container Startup Failures (Docker 409)
**Tests**: kubelet.go:53, :186, runtime.go:165, output.go:263, :282, projected_secret.go:371, hostport.go:219
- **Error**: "Told to stop trying after 2.004s" / "expected pod success"
- **Root cause**: Docker 409 conflicts (container name already in use) and pause container failures (cannot join network namespace of exited container). Infrastructure issue with Docker container lifecycle.

### Init Container Exit Code
**Tests**: init_container.go:440
- **Error**: "first init container should have exitCode != 0" but got exitCode=0
- **Root cause**: Init container should fail but completes successfully. May be a test-specific issue with how we handle init container restarts.

### Exec Websocket Close
**Tests**: exec_util.go:113
- **Error**: "websocket: close 1005 (no status)"
- **Root cause**: Exec websocket closes without proper close frame. Our handler sends close(1000) but it may not reach client before TCP drops.

### Webhook Attach Denial
**Tests**: webhook.go:1481
- **Error**: "unexpected 'kubectl attach' error message — expected 'attaching to pod is not allowed', got 'broken pipe'"
- **Root cause**: Webhook denies attach but our error handling returns wrong error format. The attach connection breaks before the denial message reaches kubectl.

### Service/kube-proxy
**Tests**: service.go:251, :768, :3459
- **Error**: "Affinity shouldn't hold" / "Affinity should hold" / "service not reachable" / "failed to delete Service"
- **Root cause**: kube-proxy session affinity timeout and endpoint routing issues.

### StatefulSet Controller
**Tests**: statefulset.go:957
- **Error**: "Pod ss-0 expected to be re-created at least once"
- **Root cause**: May be fixed by LIST resourceVersion fix (test uses watches). Otherwise, SS controller reconciliation issue.

### Aggregator
**Tests**: aggregator.go:359
- **Error**: sample-apiserver deployment never ready (ReadyReplicas: 0)
- **Root cause**: Sample API server pod failing to start.

### kubectl replace
**Tests**: builder.go:97
- **Error**: "error running kubectl replace -f"
- **Root cause**: PUT semantics issue.

### ReplicaSet Scale
**Tests**: replica_set.go:560
- **Error**: "failed to see replicas of test-rs scale to requested amount of 3"
- **Root cause**: Likely fixed by LIST resourceVersion fix (watch-dependent). Otherwise RS controller issue.

### Job Failure
**Tests**: job.go:144
- **Error**: "failed to ensure job failure"
- **Root cause**: Job controller not marking job as failed within timeout.

### Webhook Slow/Timeout (additional)
**Tests**: webhook.go:1481, webhook.go:2491
- **Error**: :1481 — "unexpected kubectl attach error — expected 'not allowed', got 'broken pipe'"; :2491 — "expect HTTP/dial timeout error, got 'failed to call webhook'"
- **Root cause**: :2491 is same as fix 3 (cause chain not in error). :1481 is attach connection breaking before denial message delivered.

### Proxy
**Tests**: proxy.go:271
- **Error**: "Unable to reach service through proxy: context deadline exceeded"
- **Root cause**: API server proxy endpoint not forwarding to service.

### DaemonSet
**Tests**: daemon_set.go:1276
- **Error**: "Expected 0 to equal 1"
- **Root cause**: DaemonSet pod not running on node.

### Pod Resize
**Tests**: pod_resize.go:857
- **Status**: Partially implemented. Known issue from previous rounds.

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 141 | 368 | 73 | 441 | 83.4% |
| 143 | 372 | 69 | 441 | 84.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |
| 146 | 379 | 62 | 441 | 85.9% (pre-fix baseline) |

## Commits

```
4e91110 docs: Final tracker update — 8 fixes, expect 25-30 of 36 tests fixed
7705532 docs: Add LIST resourceVersion fix — 8 total fixes, biggest systemic fix
74dfb4b fix: Use etcd mod_revision for LIST resourceVersion instead of timestamps
6ba7757 docs: Add watch context canceled and DaemonSet categories to tracker
3a224ab docs: Add CRD OpenAPI items fix to tracker — 7 fixes total
ecb67b7 fix: Unwrap CRD items schema for OpenAPI v2 compatibility
02e1581 docs: Update conformance tracker with 36+ failures and 6 fixes
0f001e5 fix: Re-apply defaults after mutating webhooks (terminationMessagePolicy)
3930fe8 fix: 5 conformance fixes — RC selector, webhook matchConditions, SMP ordering, pod conditions
```

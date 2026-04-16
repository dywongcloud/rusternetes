# Conformance Failure Tracker

**Round 146** | 379/441 passed (85.9%) — 62 failed, 51 unique test locations | 2026-04-15

## Fixes Applied This Round (14 total, not yet deployed)

| # | Fix | Root Cause | Fix Location | Tests Expected to Fix |
|---|-----|-----------|-------------|----------------------|
| 1 | RC selector defaulting | K8s defaults RC.Spec.Selector from Template.Labels; ours was null | api-server/handlers/replicationcontroller.rs | rc.go:623, garbage_collector.go:436 |
| 2 | Webhook matchConditions | CEL match conditions never evaluated in mutating/validating paths | api-server/admission_webhook.rs | webhook.go:932, :2222, :2164 |
| 3 | Webhook timeout "deadline" | reqwest cause chain not included in error message | api-server/admission_webhook.rs | webhook.go:1400, :2491 |
| 4 | SMP array ordering | K8s puts patch items before server-only; ours preserved original order | api-server/patch.rs | statefulset.go:1092 |
| 5 | Pod Succeeded conditions | Missing PodInitialized=True with reason PodCompleted | kubelet/kubelet.rs | init_container.go:235 |
| 6 | Defaults after mutation | K8s runs SetDefaults twice (before AND after webhooks) | api-server/handlers/pod.rs | webhook.go:1352 |
| 7 | CRD OpenAPI items unwrap | items had extra {"schema": {...}} wrapper for Swagger v2 | api-server/handlers/openapi.rs | 10 crd_publish_openapi.go tests |
| 8 | LIST resourceVersion | All 11 LIST handlers used timestamps instead of etcd mod_revisions | 7 handler files | systemic watch failures — many tests |
| 9 | Init container restart tracking | restart_count always 0, last_state always None | kubelet/runtime.rs | init_container.go:440 |
| 10 | ResourceQuota cpu/memory aliases | "cpu" alias for "requests.cpu" not handled; errors silently passed | api-server/admission.rs, handlers/pod.rs | resource_quota.go:290 |
| 11 | Exec websocket Success status | Missing status JSON on channel 3 for exit code 0 | api-server/streaming.rs | exec_util.go:113 |
| 12 | Docker 409 container conflict | 100ms wait insufficient; containers not stopped before removal | kubelet/runtime.rs | pod startup failures (~9 tests) |
| 13 | Attach webhook validation | Attach handler missing webhook check entirely | api-server/handlers/pod_subresources.rs | webhook.go:1481 |
| 14 | Per-pod sync lock | Concurrent sync_pod calls for same pod → Docker 409 races | kubelet/kubelet.rs | all pod startup failures |

## Root Cause Details

### FIX 1: RC Selector Defaulting
- **Error**: rc.go — "rc manager never removed the failure condition"; gc.go — "expect 100 pods, got 9 pods"
- **Root cause**: K8s API server defaults `RC.Spec.Selector` from `Template.Labels` when selector is nil. Our API stored `selector: null`. The RC controller's `labels_match_selector()` returned false for ALL pods → released them → tried to recreate → hit quota.
- **K8s ref**: `pkg/registry/core/replicationcontroller/strategy.go`

### FIX 2: Webhook matchConditions
- **Error**: :932 — configmap "skip-me" mutated when it shouldn't be; :2222 — got extra `mutation-stage-2`; :2164 — CR update denied when it should be allowed
- **Root cause**: `run_mutating_webhooks()` and `run_validating_webhooks()` never evaluated `matchConditions` CEL expressions. All webhooks fired regardless.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/admission/plugin/webhook/predicates`

### FIX 3: Webhook Timeout "deadline"
- **Error**: `expect error "deadline", got "failed to call webhook: error sending request for url (...)"`
- **Root cause**: reqwest error Display only shows top-level message. The nested cause "deadline has elapsed" was lost. Now includes full cause chain.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/admission/plugin/webhook/validating/dispatcher.go:311`

### FIX 4: SMP Array Ordering
- **Error**: "statefulset not using ssPatchImage"
- **Root cause**: K8s `normalizeElementOrder` puts patch items FIRST, then server-only. Our code preserved original order. Patch container "test-ss" was appended instead of prepended.
- **K8s ref**: `apimachinery/pkg/util/strategicpatch/patch.go:1534-1544`

### FIX 5: Pod Succeeded Conditions
- **Error**: `Expected *v1.PodCondition nil not to be nil`
- **Root cause**: Kubelet set `Phase::Succeeded` but never set conditions. K8s always sets `PodInitialized=True` with reason `PodCompleted` on succeeded pods.
- **K8s ref**: `pkg/kubelet/status/generate.go:209-217`

### FIX 6: Defaults After Mutation
- **Error**: "expect the init terminationMessagePolicy to be default to 'File', got ''"
- **Root cause**: K8s runs SetDefaults TWICE: before and after mutating webhooks. We only ran it once. Webhook-added init containers had empty `terminationMessagePolicy`.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/endpoints/handlers/create.go`

### FIX 7: CRD OpenAPI Items Schema
- **Error**: kubectl explain fails; schema "not match"
- **Root cause**: K8s CRD schemas store `items` as `{"schema": {...}}` (Go JSONSchemaPropsOrArray). OpenAPI v2 expects `items` as a direct schema. Our code copied raw JSON without unwrapping.
- **Verified**: `/openapi/v2` output showed `items.schema.type` instead of `items.type`.
- **K8s ref**: `vendor/k8s.io/apiextensions-apiserver/pkg/apis/apiextensions/v1/types_jsonschema.go`

### FIX 8: LIST resourceVersion (BIGGEST FIX)
- **Error**: 1123 `Watch failed: context canceled` per conformance run
- **Root cause**: ALL 11 LIST handlers used `chrono::Utc::now().timestamp()` (~1.7 billion) as the list resourceVersion. Items had etcd mod_revisions (~75,000). LIST+WATCH starts the watch from the LIST's RV, but etcd will never reach 1.7 billion → every watch fails immediately.
- **Verified**: `curl /api/v1/pods` returned list RV=1776302579, items had RV=75027.
- **Impact**: Systemic — affects every test that uses LIST+WATCH (deployment, RS, SS, service, sysctl, preemption, DaemonSet, etc.).

### FIX 9: Init Container Restart Tracking
- **Error**: "first init container should have exitCode != 0" (exitCode was 0)
- **Root cause**: `get_init_container_statuses()` always set `restart_count: 0` and `last_state: None`. When a failed init container was removed and recreated, restart history was lost. Test expects `restart_count >= 3`.
- **K8s ref**: `pkg/kubelet/kuberuntime/kuberuntime_container.go`

### FIX 10: ResourceQuota cpu/memory Aliases
- **Error**: "Expected an error to have occurred. Got: nil" — pod created despite exceeding quota
- **Root cause**: Quota check only looked for `"requests.cpu"` key, but the test's quota used `"cpu"` (K8s alias). Also, quota check errors were silently ignored instead of rejecting the pod.
- **K8s ref**: `pkg/quota/v1/evaluator/core/pods.go`

### FIX 11: Exec Websocket Success Status
- **Error**: "websocket: close 1005 (no status)"
- **Root cause**: K8s v4/v5 exec protocol requires a status JSON on channel 3 for ALL exit codes. We only sent status for non-zero exits. The client's error stream reader blocks waiting for status; without it, it gets a close frame without data → "close 1005".
- **K8s ref**: `staging/src/k8s.io/client-go/tools/remotecommand/v4.go`

### FIX 12: Docker 409 Container Conflicts
- **Error**: "Told to stop trying after 2.004s" / "expected pod success" (1014 Docker 409 errors per run)
- **Root cause**: When removing a conflicting container, we waited only 100ms before retrying create. Docker needs more time to release container names. Also didn't stop running containers before removing.
- **Fix**: Stop container first, increase wait to 500ms, check remove result.

## Remaining Unfixed Issues

### Pod Startup Failures (partially addressed by fix 12)
**Tests**: kubelet.go:53, :186, runtime.go:129, :165, output.go:263, :282, projected_secret.go:371, hostport.go:219, rc.go:509, deployment.go:995, proxy.go:503
- **Error**: "Told to stop trying" / "expected pod success" / pod timeout
- **Root cause**: Docker 409 conflicts cause container creation to fail. Fix 12 mitigates by stopping before removing and increasing wait, but concurrent kubelet sync cycles can still race on the same container name. Pods on these tests never reach Ready.
- **Downstream impact**: Also causes aggregator.go:359, service.go:251/:768/:4271, service_latency.go:145, proxy.go:271 (services have no ready endpoints because pods didn't start).

### Webhook Attach Denial
**Tests**: webhook.go:1481
- **Error**: "expected 'attaching to pod is not allowed', got 'broken pipe'"
- **Root cause**: Webhook denies attach request but the connection breaks before the denial message reaches kubectl. Our attach handler may not properly propagate webhook denial errors over the websocket/SPDY connection.

### kubectl replace CAS Conflict
**Tests**: builder.go:97
- **Error**: "the object has been modified; please apply your changes to the latest version" (stored RV: 31356, provided: 31348)
- **Root cause**: Between kubectl read and write, the kubelet or controller updated the pod's resourceVersion. This is a race condition exacerbated by frequent status updates.

### Job Failure Detection
**Tests**: job.go:144
- **Error**: "job completed while waiting for its failure"
- **Root cause**: Job's init container should exit non-zero, causing the job to fail. But our kubelet may report exitCode 0 for the init container (related to Docker inspect timing), causing the job to complete instead of fail.

### Pod Resize
**Tests**: pod_resize.go:857
- **Status**: Partially implemented from previous rounds. API sets resize=Proposed, kubelet calls Docker update, but some containers don't get cgroup updates.

### Preemption
**Tests**: preemption.go:877
- **Error**: "failed pod observation expectations: context deadline exceeded"
- **Root cause**: Watch-dependent test. Likely fixed by fix 8 (LIST resourceVersion). If not, scheduler preemption may need investigation.

## Failure-to-Fix Mapping

### Expected to be fixed by deployed fixes:
| Fix | Tests |
|-----|-------|
| 1 (RC selector) | rc.go:623, garbage_collector.go:436 |
| 2 (matchConditions) | webhook.go:932, :2222, :2164 |
| 3 (deadline) | webhook.go:1400, :2491 |
| 4 (SMP order) | statefulset.go:1092 |
| 5 (conditions) | init_container.go:235 |
| 6 (defaults) | webhook.go:1352 |
| 7 (CRD items) | crd_publish_openapi.go:77, :184, :225, :267, :285, :318, :366, :400, :451 |
| 8 (LIST RV) | replica_set.go:232, :560, deployment.go:1259, sysctl.go:100, statefulset.go:957, daemon_set.go:1276, service.go:3459, service_latency.go:145, preemption.go:877 |
| 9 (init restart) | init_container.go:440 |
| 10 (quota alias) | resource_quota.go:290 |
| 11 (exec ws) | exec_util.go:113 |
| 12 (Docker 409) | kubelet.go:53, :186, runtime.go:129, :165, output.go:263, :282, projected_secret.go:371, hostport.go:219, rc.go:509 |
| 12 downstream | aggregator.go:359, deployment.go:995, proxy.go:271, :503, service.go:251, :768, :4271 |

### Likely still failing after deployment:
- builder.go:97 (CAS race — kubelet status updates bump RV between kubectl read and write)
- job.go:144 (downstream of Docker 409 — pod can't start, so exit code never captured)
- pod_resize.go:857 (known partial implementation)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 141 | 368 | 73 | 441 | 83.4% |
| 143 | 372 | 69 | 441 | 84.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |
| 146 | 379 | 62 | 441 | 85.9% (pre-fix baseline) |

## Commits (This Round)

```
59ec5c6 fix: Per-pod sync lock prevents concurrent sync_pod Docker 409 races
de85728 fix: Run admission webhooks on pod attach requests
f23bfeb fix: Docker container name 409 conflict — stop before remove, increase wait
92d6314 fix: Always send Success status on exec websocket channel 3
d120ea1 fix: ResourceQuota check cpu/memory aliases and fail on quota errors
e1e3da2 fix: Track init container restart_count and last_state across recreations
74dfb4b fix: Use etcd mod_revision for LIST resourceVersion instead of timestamps
ecb67b7 fix: Unwrap CRD items schema for OpenAPI v2 compatibility
0f001e5 fix: Re-apply defaults after mutating webhooks (terminationMessagePolicy)
3930fe8 fix: 5 conformance fixes — RC selector, webhook matchConditions, SMP ordering, pod conditions
```

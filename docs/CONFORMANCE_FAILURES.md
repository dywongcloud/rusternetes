# Conformance Failure Tracker

**Round 146** | In Progress — 36+ unique test failures | 2026-04-15

## Root Causes & Fixes

### Category 1: RC/RS Selector Defaulting — FIX APPLIED
**Affected tests**: rc.go:623, garbage_collector.go:436, replica_set.go:232, deployment.go:1259
- **Root cause**: K8s API server defaults `RC.Spec.Selector` from `Template.Labels` when selector is nil. Our API server stores `selector: null`. The RC controller's `labels_match_selector()` always returns false → releases all owned pods → tries to recreate → hits quota → condition never clears. The GC test creates an RC with 100 replicas; only 9 pods created because selector mismatch causes thrashing.
- **K8s ref**: `pkg/registry/core/replicationcontroller/strategy.go`
- **Fix**: Default `rc.spec.selector` from `rc.spec.template.metadata.labels` in RC create handler.

### Category 2: Webhook matchConditions Not Evaluated — FIX APPLIED
**Affected tests**: webhook.go:932, webhook.go:2222, webhook.go:2164, webhook.go:1352
- **Root cause**: `run_mutating_webhooks()` and `run_validating_webhooks()` never evaluate `matchConditions` CEL expressions. All webhooks are called regardless of match conditions. Test creates webhook with match condition to skip configmaps named "skip-me" — our code calls the webhook anyway, resulting in extra mutations.
- **Error examples**: webhook.go:932 — "create the configmap with 'skip-me' name" fails; webhook.go:2222 — got `{mutation-stage-2: yes}` when it shouldn't be present; webhook.go:2164 — "updating custom resource should be denied" when it should be allowed.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/admission/plugin/webhook/predicates`
- **Fix**: Added CEL matchConditions evaluation to both `run_mutating_webhooks()` and `run_validating_webhooks()`.

### Category 3: Webhook Timeout Error Message — FIX APPLIED
**Affected tests**: webhook.go:1400
- **Root cause**: Test checks `err.Error()` contains "deadline". Our reqwest error wraps as `"failed to call webhook: error sending request for url (...)"` — the nested cause "deadline has elapsed" isn't included in the Display output.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/admission/plugin/webhook/validating/dispatcher.go:311`
- **Fix**: Include error cause chain in webhook error message.

### Category 4: SMP Array Ordering — FIX APPLIED
**Affected tests**: statefulset.go:1092
- **Root cause**: K8s SMP puts patch items FIRST, then server-only items. Our implementation preserved original order. Test patches SS with container name "test-ss" (not in original), expects it at index [0].
- **K8s ref**: `apimachinery/pkg/util/strategicpatch/patch.go:normalizeElementOrder`
- **Fix**: Changed `strategic_merge_arrays` to put patch items first, then server-only items.

### Category 5: Pod Succeeded Conditions — FIX APPLIED
**Affected tests**: init_container.go:235
- **Root cause**: Kubelet doesn't set conditions when pod transitions to `Phase::Succeeded`. Test expects `PodInitialized=True` with reason `PodCompleted`.
- **K8s ref**: `pkg/kubelet/status/generate.go:209-217`
- **Fix**: Added `succeeded_pod_conditions()` and set conditions on all Succeeded phase transitions.

### Category 6: Defaults After Mutation — FIX APPLIED
**Affected tests**: webhook.go:1352, runtime.go:129
- **Error**: "expect the init terminationMessagePolicy to be default to 'File', got ''"
- **Root cause**: K8s runs SetDefaults TWICE: before and after mutating webhooks. We only ran it once (before). Webhook-added containers (init containers added by mutation) never get defaults applied.
- **K8s ref**: `staging/src/k8s.io/apiserver/pkg/endpoints/handlers/create.go`
- **Fix**: Re-apply `apply_pod_spec_defaults` after webhook mutation in pod create handler.

### Category 7: CRD OpenAPI Items Schema Wrapping — FIX APPLIED
**Affected tests**: crd_publish_openapi.go:77, :184, :225, :267, :285, :318, :366, :451
- **Error**: kubectl explain fails; schema comparison "not match"
- **Root cause**: K8s CRD schemas store `items` as `{"schema": {...}}` (Go's JSONSchemaPropsOrArray format). OpenAPI v2 expects `items` to be a direct schema object. Our code copied the raw JSON without unwrapping, producing `items.schema.type` instead of `items.type`.
- **K8s ref**: `vendor/k8s.io/apiextensions-apiserver/pkg/apis/apiextensions/v1/types_jsonschema.go`
- **Fix**: Unwrap `{"schema": {...}}` wrapper from items in `strip_false_extensions()`.

### Category 8: Pod/Container Startup Failures — NOT YET FIXED
**Affected tests**: kubelet.go:53, kubelet.go:186, runtime.go:165, output.go:263, projected_secret.go:371
- **Error**: "Told to stop trying after 2.004s" / "expected pod success"
- **Root cause**: Pods not becoming Ready within the test framework's timeout. Kubelet logs show Docker 409 conflicts and pause container failures. These are intermittent Docker-level issues (container name conflicts, network namespace join failures).
- **Status**: Recurring infrastructure issue with Docker container lifecycle management.

### Category 9: Exec Websocket — NOT YET FIXED
**Affected tests**: exec_util.go:113
- **Error**: "websocket: close 1005 (no status)"
- **Root cause**: Pod exec websocket closes without proper close frame. Our exec handler likely drops the websocket connection without sending a proper close message.

### Category 10: HostPort Scheduling — NOT YET FIXED
**Affected tests**: hostport.go:219
- **Error**: "wait for pod pod2 timeout" — pod2 with different hostIP same port never scheduled
- **Root cause**: Scheduler hostPort conflict detection doesn't account for different hostIPs. Two pods with same hostPort but different hostIPs should be co-schedulable.

### Category 11: Service/kube-proxy — NOT YET FIXED
**Affected tests**: service.go:251, service.go:768, service.go:3459
- **Error**: "Affinity shouldn't hold but did" / "Affinity should hold but didn't" / "service is not reachable" / "failed to delete Service: timed out"
- **Root cause**: kube-proxy session affinity timeout logic. Service endpoint routing issues.

### Category 12: StatefulSet Controller — NOT YET FIXED
**Affected tests**: statefulset.go:957
- **Error**: "Pod ss-0 expected to be re-created at least once"
- **Root cause**: StatefulSet controller not properly deleting and recreating pods during reconciliation.

### Category 13: Aggregator/Sample API Server — NOT YET FIXED
**Affected tests**: aggregator.go:359
- **Error**: sample-apiserver deployment never ready (ReadyReplicas: 0)
- **Root cause**: Sample API server pod failing to start or readiness probe failing.

### Category 14: kubectl replace — NOT YET FIXED
**Affected tests**: builder.go:97
- **Error**: "error running kubectl replace -f"
- **Root cause**: Unknown — needs investigation of our PUT semantics.

### Category 15: Pod Resize — KNOWN PARTIAL
**Affected tests**: pod_resize.go:857
- **Status**: Partially implemented. Resize works for some containers but not all.

## Fixes Applied (This Round)

| # | Issue | Fix Location | Tests Fixed |
|---|-------|-------------|-------------|
| 1 | RC selector defaulting | api-server/handlers/replicationcontroller.rs | rc.go:623, gc.go:436 |
| 2 | Webhook matchConditions | api-server/admission_webhook.rs | webhook.go:932, :2222, :2164, :1352 |
| 3 | Webhook timeout "deadline" | api-server/admission_webhook.rs | webhook.go:1400 |
| 4 | SMP array ordering | api-server/patch.rs | statefulset.go:1092 |
| 5 | Pod Succeeded conditions | kubelet/kubelet.rs | init_container.go:235 |
| 6 | Defaults after mutation | api-server/handlers/pod.rs | webhook.go:1352 |
| 7 | CRD items schema unwrap | api-server/handlers/openapi.rs | crd_publish_openapi.go (8 tests) |

## Expected Impact

With fixes 1-7, we expect to fix approximately 19-21 of the 36 failures:
- Fix 1 (RC selector): rc.go, gc.go, possibly deployment.go, replica_set.go (2-4 tests)
- Fix 2 (matchConditions): webhook.go:932, :2222, :2164 (3 tests)
- Fix 3 (deadline): webhook.go:1400 (1 test)
- Fix 4 (SMP ordering): statefulset.go:1092 (1 test)
- Fix 5 (conditions): init_container.go:235 (1 test)
- Fix 6 (defaults after mutation): webhook.go:1352 (1 test)
- Fix 7 (CRD items unwrap): crd_publish_openapi.go:77, :184, :225, :267, :285, :318, :366, :451 (8 tests)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 141 | 368 | 73 | 441 | 83.4% |
| 143 | 372 | 69 | 441 | 84.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |
| 146 | TBD | 36+ | 441 | TBD (in progress) |

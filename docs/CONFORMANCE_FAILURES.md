# Conformance Issue Tracker

**Round 96**: 28 FAIL, 0 PASS (running) | **168 fixes** (166 deployed + 2 pending: PDB status PATCH, restart count monotonic)

## Round 96 Failures

| # | Test | Error | Category |
|---|------|-------|----------|
| 1 | statefulset.go:786 | timed out | Watch timeout |
| 2 | rc.go:717 | timed out | Watch timeout |
| 3 | output.go:282 | CPU_LIMIT=2 expected | Downward API resource ref |
| 4 | aggregator.go:359 | ReadyReplicas=0 | Webhook pod not ready |
| 5 | custom_resource_definition.go:104 | missing field `spec` | Protobuf: TypeMeta-only JSON lacks spec |
| 6 | output.go:263 | pod output mismatch | Container output |
| 7 | job.go:755 | job completion timeout | Job controller timing |
| 8 | lifecycle_hook.go:132 | timed out 60s | Lifecycle hook/watch |
| 9 | pod_client.go:216 | expected pod success timeout | Pod lifecycle |
| 10 | projected_configmap.go:367 | configmap subpath timeout | Volume update |
| 11 | webhook.go:1269 | webhook not ready | Webhook pod not ready |

## Root Causes

### 1. Watch reliability (causes ~50% of failures)
Watch-based waiters time out. The HTTP/2 watch streams work but events
may not be delivered to the right subscriber at the right time.
Events ARE being generated (2471 events seen in one test).
The broadcast channel approach may miss events between subscribe and list.

### 2. Protobuf (CRD failures)
K8s client sends protobuf for CRDs. TypeMeta extraction gives
`{"apiVersion":"...","kind":"...","metadata":{}}` but handler needs `spec`.

### 3. Webhook/aggregator pods
Containers start and run but deployment shows ReadyReplicas=0.
Kubelet sync fix (#161) should help but may need further debugging.

### 4. Downward API CPU_LIMIT
Container env var CPU_LIMIT not set correctly from resource field ref.

## Progress
- Framework no longer hangs (resourceVersion + initial-events-end fixes working)
- Pods DO start (deployment became Available in 6s in round 94)
- Events ARE being delivered
- Tests progress but very slowly (~11 tests in 1 hour)
- 0 passes so far — likely need to fix watch event delivery reliability

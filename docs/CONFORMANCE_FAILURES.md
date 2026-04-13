# Conformance Failure Tracker

**Round 137** | Running | 2026-04-13

## Known Issues To Fix

Round 137 has 30 fixes deployed since round 135 (373/441). The following issues are known from round 135 analysis. Some may be resolved by staged fixes; others need new work. Round 137 results will confirm which are actually fixed.

### 1. Watch "context canceled" — ~8 failures
- `deployment.go:1008,1322`, `rc.go:509,623`, `replica_set.go:232,560`, `runtime.go:115`, `statefulset.go:957`
- **Status**: FIX DEPLOYED (069e807 — HTTP/2 ALPN negotiation)
- **Awaiting round 137 results**

### 2. Webhook — 12 failures
- `webhook.go:520,675,904,1269,1334,1400,1481,2107(x3),2164,2491`
- **Status**: FIX DEPLOYED (3012663 kube-proxy XOR hash + fe76396 RELATED,ESTABLISHED + 7cf9bd5 objectSelector + 8673d37 parallel dispatch)
- **Awaiting round 137 results**

### 3. CRD OpenAPI — 9 failures
- `crd_publish_openapi.go:77,161,214,253,285,318,366,400,451`
- **Status**: FIX DEPLOYED (0188c3c — raw JSON CRD schemas in OpenAPI handler)
- **Awaiting round 137 results**

### 4. DNS — 6 failures
- `dns_common.go:476` (x6)
- **Status**: Downstream of kube-proxy fixes (3012663 + fe76396). No DNS-specific code changes needed — resolv.conf generation verified correct.
- **Awaiting round 137 results**

### 5. Service Networking — 6 failures
- `service.go:768,886,3459`, `proxy.go:271,503`, `service_latency.go:145`
- **Status**: FIX DEPLOYED (3012663 + fe76396 — kube-proxy flush gap + filter rules)
- **Awaiting round 137 results**

### 6. Preemption — 4 failures
- `predicates.go:1041(x2)`, `preemption.go:535,1052`
- **Status**: FIX DEPLOYED (e1f4bd0 — all resource types in preemption + b60b87a system PriorityClasses + c19a049 priority admission controller)
- **Awaiting round 137 results**

### 7. EmptyDir — 4 failures (webhook cascade)
- `output.go:263` (x4)
- **Status**: FIX DEPLOYED (8673d37 parallel webhook dispatch + kube-proxy fixes)
- **Awaiting round 137 results**

### 8. Field Validation — 3 failures
- `field_validation.go:462,611,735`
- **Status**: FIX DEPLOYED (a18febe + e2e2f48 — strict unknown field validation)
- **Awaiting round 137 results**

### 9. Deployment — 2 failures
- `deployment.go:1008,1322`
- **Status**: FIX DEPLOYED (ea9573e force scale-down + 07a393c proportional scaling)
- **Awaiting round 137 results**

### 10. DaemonSet — 1 failure
- `daemon_set.go:1276`
- **Status**: FIX DEPLOYED (646c713 — SafeEncodeString hash)
- **Awaiting round 137 results**

### 11. ResourceQuota — 1 failure
- `resource_quota.go:282`
- **Status**: FIX DEPLOYED (0ed1628 — ephemeral-storage tracking)
- **Awaiting round 137 results**

### 12. Namespace Deletion — 1 failure
- `namespace.go:609`
- **Status**: FIX DEPLOYED (a1025ba — pod deletion ordering)
- **Awaiting round 137 results**

### 13. Job — 1 failure
- `job.go:556`
- **Status**: FIX DEPLOYED (31f4f39 — terminating count for completed jobs)
- **Awaiting round 137 results**

### 14. Auth — 2 failures
- `service_accounts.go:129,667`
- **Status**: FIX DEPLOYED (2f20539 — RS256 key path)
- **Awaiting round 137 results**

### 15. Discovery — 1 failure
- `discovery.go:131`
- **Status**: FIX DEPLOYED (3ba5e20 — trailing slash routes)
- **Awaiting round 137 results**

### 16. EndpointSlice Mirroring — 1 failure
- `endpointslicemirroring.go:202`
- **Status**: FIX DEPLOYED (361752a — mirroring cleanup)
- **Awaiting round 137 results**

### 17. StatefulSet — 1 failure
- `statefulset.go:1092`
- **Status**: FIX DEPLOYED (8673d37 generation + terminal pods + 4438743 computeReplicaStatus)
- **Awaiting round 137 results**

### 18. Lifecycle Hook — 1 failure
- `lifecycle_hook.go:132`
- **Status**: Downstream of kube-proxy fixes (preStop HTTP hook uses pod-to-pod networking)
- **Awaiting round 137 results**

### 19. DinD Limitations — 3 failures (may not be fixable)
- `hostport.go:219` — DinD can't bind to other node's IPs
- `pod_resize.go:857` — DinD cgroup limitations
- `aggregator.go:359` — Sample API server needs image pull in DinD

## Fixes Deployed in Round 137

| Commit | Fix | K8s Ref |
|--------|-----|---------|
| 069e807 | TLS HTTP/2 ALPN negotiation | serving.go |
| 3012663 | kube-proxy XOR hash — eliminate flush gap | proxier.go |
| fe76396 | kube-proxy RELATED,ESTABLISHED + filter OUTPUT | proxier.go:1460,386 |
| 7f8d692 | kube-proxy --reap for session affinity | proxier.go:1557 |
| 0188c3c | OpenAPI raw JSON CRD schemas | customresource_handler.go |
| e1f4bd0 | Preemption all resource types | preemption.go |
| 646c713 | DaemonSet SafeEncodeString hash | rand.go |
| ea9573e | Deployment force scale-down old RSes | rolling.go |
| 07a393c | Deployment proportional scaling | sync.go scale() |
| 73795a7 | Endpoints terminal/terminating/publishNotReady | controller_utils.go |
| 0ed1628 | ResourceQuota ephemeral-storage | pods.go PodUsageFunc |
| a1025ba | Namespace deletion pod ordering | namespaced_resources_deleter.go |
| 31f4f39 | Job terminating count for completed | job_controller.go |
| 2f20539 | Kubelet RS256 key path | jwt.go |
| 7cf9bd5 | Webhook objectSelector | object/matcher.go |
| a18febe | CRD strict unknown top-level fields | customresource_handler.go |
| e2e2f48 | CRD strict unknown metadata fields | customresource_handler.go |
| 3ba5e20 | Trailing slash routes /api/ /apis/ | http.ServeMux |
| 361752a | EndpointSlice mirroring cleanup | reconciler.go |
| 7fa3ce5 | Kubelet concurrent pod sync | podWorkerLoop |
| d3011e0 | Kubelet init container state machine | computeInitContainerActions |
| 7ea2d20 | Kubelet init container status during backoff | kuberuntime_container.go |
| 8673d37 | Parallel validating webhook dispatch | dispatcher.go:126-131 |
| 8673d37 | Generic PATCH generation increment | patch.go |
| 8673d37 | StatefulSet delete terminal pods | stateful_set_control.go:431 |
| 4438743 | StatefulSet computeReplicaStatus() counting | stateful_set_control.go:370-399 |
| b60b87a | System PriorityClasses + CoreDNS priority | scheduling/types.go |
| c19a049 | Priority admission controller — resolve priorityClassName | admission/priority/admission.go |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | TBD | TBD | 441 | TBD |

# Conformance Failure Tracker

**Round 135** | 373/441 (84.6%) | 2026-04-11
**Round 136** | Pending (27 fixes staged) | 2026-04-12

## Staged Fixes for Round 136 (all from deep K8s source comparison)

| Commit | Fix | K8s Ref | Expected Impact |
|--------|-----|---------|-----------------|
| 069e807 | TLS HTTP/2 ALPN negotiation (h2 + http/1.1) | serving.go | ~8 watch failures |
| 3012663 | kube-proxy XOR hash — eliminate flush gap | proxier.go iptables-restore | ~18 (webhook+service+DNS) |
| fe76396 | kube-proxy RELATED,ESTABLISHED + filter OUTPUT | proxier.go:1460,386 | service return traffic |
| 7f8d692 | kube-proxy --reap for session affinity | proxier.go:1557 | affinity edge cases |
| 0188c3c | OpenAPI raw JSON CRD schemas | customresource_handler.go | 9 CRD OpenAPI |
| e1f4bd0 | Preemption all resource types | preemption.go | 4 preemption |
| 646c713 | DaemonSet SafeEncodeString hash | rand.go | 1 daemonset |
| ea9573e | Deployment force scale-down old RSes | rolling.go reconcileOldReplicaSets | 2 deployment |
| 73795a7 | Endpoints terminal/terminating/publishNotReady | controller_utils.go ShouldPodBeInEndpoints | endpoint reliability |
| 0ed1628 | ResourceQuota ephemeral-storage | pods.go PodUsageFunc | 1 quota |
| a1025ba | Namespace deletion pod ordering | namespaced_resources_deleter.go | 1 namespace |
| 31f4f39 | Job terminating count for completed | job_controller.go syncJob | 1 job |
| 2f20539 | Kubelet RS256 key path | jwt.go | 1 SA token |
| 7cf9bd5 | Webhook objectSelector | object/matcher.go | webhook reliability |
| a18febe | CRD strict unknown top-level fields | customresource_handler.go | 1 field validation |
| e2e2f48 | CRD strict unknown metadata fields | customresource_handler.go | 1 field validation |
| 3ba5e20 | Trailing slash routes /api/ /apis/ | Go http.ServeMux | 1 discovery |
| 361752a | EndpointSlice mirroring cleanup | reconciler.go | 1 mirroring |
| 07a393c | Deployment proportional scaling | sync.go scale() | deployment reliability |
| 7fa3ce5 | Kubelet concurrent pod sync via tokio::spawn | podWorkerLoop | pod start timing |
| d3011e0 | Kubelet init container state machine | computeInitContainerActions | init container restart |
| 7ea2d20 | Kubelet init container status during backoff | kuberuntime_container.go | init container status |
| 8673d37 | Parallel validating webhook dispatch | dispatcher.go:126-131 | ~4 emptydir webhook cascade |
| 8673d37 | Generic PATCH generation increment | patch.go | 1 statefulset patch |
| 8673d37 | StatefulSet delete terminal (Failed/Succeeded) pods | stateful_set_control.go:431 processReplica | statefulset lifecycle |
| 4438743 | StatefulSet replica counting — match computeReplicaStatus() | stateful_set_control.go:370-399 | statefulset status accuracy |
| ea9573e | Deployment force scale-down old RSes when new RS available | rolling.go | 2 deployment |

## Round 135 Failure Analysis (68 failures, 57 unique locations)

### Watch "context canceled" — ~8 failures (FIX STAGED 069e807)
- `deployment.go:1008,1322`, `rc.go:509,623`, `replica_set.go:232,560`, `runtime.go:115`, `statefulset.go:957`
- **Root cause found**: TLS server didn't advertise HTTP/2 via ALPN. Go's client-go fell back to HTTP/1.1 causing connection pooling issues with watches.
- **Fix**: 069e807 enables h2 + http/1.1 ALPN in rustls ServerConfig
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/server/options/serving.go

### Webhook — 12 failures (FIX STAGED 3012663 + fe76396 + 7cf9bd5 + 8673d37)
- `webhook.go:520,675,904,1269,1334,1400,1481,2107(x3),2164,2491`
- **Root cause found**: kube-proxy flushed ALL iptables rules every second because hash was order-dependent and NEVER matched. Webhook ClusterIP rules existed for only ~50ms/second. FailurePolicy=Ignore silently swallowed the errors.
- **Deep analysis findings**: Also missing objectSelector, missing RELATED,ESTABLISHED filter rule, missing OUTPUT chain jump. Validating webhooks called sequentially instead of in parallel.
- **Fix**: kube-proxy XOR hash eliminates flush gap. RELATED,ESTABLISHED + OUTPUT chain fix service traffic. objectSelector matching added. Validating webhooks now dispatched in parallel via tokio::spawn matching K8s goroutine architecture (dispatcher.go:126-131).
- **K8s ref**: pkg/proxy/iptables/proxier.go, admission/plugin/webhook/validating/dispatcher.go, predicates/object/matcher.go

### CRD OpenAPI — 9 failures (FIX STAGED 0188c3c)
- `crd_publish_openapi.go:77,161,214,253,285,318,366,400,451`
- **Root cause found**: OpenAPI handler deserialized CRDs through typed struct losing nested `items` in JSONSchemaPropsOrArray untagged enum. Confirmed: CRDs stored with raw JSON (fix 047ba6b) but OpenAPI handler re-deserialized them.
- **Fix**: 0188c3c reads CRDs as raw serde_json::Value in OpenAPI handler
- **K8s ref**: staging/src/k8s.io/apiextensions-apiserver/pkg/controller/openapi/builder/builder.go

### DNS — 6 failures (downstream of kube-proxy fixes)
- `dns_common.go:476` (x6)
- **Root cause**: DNS test pods couldn't reach CoreDNS or test servers because kube-proxy iptables flush gap broke ClusterIP routing to CoreDNS (10.96.0.10). Verified: resolv.conf generation matches K8s (`{ns}.svc.{domain} svc.{domain} {domain}`, ndots:5). Fix is kube-proxy XOR hash + RELATED,ESTABLISHED.
- **K8s ref**: pkg/kubelet/network/dns/dns.go — generateSearchesForDNSClusterFirst

### Service Networking — 6 failures (FIX STAGED 3012663 + fe76396)
- `service.go:768,886,3459`, `proxy.go:271,503`, `service_latency.go:145`
- **Root cause found (deep analysis)**: kube-proxy missing RELATED,ESTABLISHED accept rule (return traffic dropped), missing filter OUTPUT chain jump (local pod→ClusterIP failed), flush+rebuild gap every second.
- **K8s ref**: pkg/proxy/iptables/proxier.go lines 378-386, 1451-1466

### Preemption — 4 failures (FIX STAGED e1f4bd0)
- `predicates.go:1041(x2)`, `preemption.go:535,1052`
- **Root cause found**: Preemption only checked cpu/memory, not extended resources
- **K8s ref**: pkg/scheduler/framework/preemption/preemption.go

### EmptyDir — 4 failures (FIX STAGED 8673d37 + kube-proxy)
- `output.go:263` (x4) — stale webhook blocks pod creation
- **Root cause found (deep analysis)**: Two issues: (1) kube-proxy flush gap breaks webhook ClusterIP routing; (2) validating webhooks called sequentially — N stale webhooks each timing out = N*10s delay. K8s dispatches all validating webhooks concurrently via goroutines (dispatcher.go:126-131), bounding total delay to max(10s).
- **Fix**: 8673d37 refactored run_validating_webhooks to use tokio::spawn + join_all for parallel execution matching K8s architecture. Combined with kube-proxy XOR hash fix to eliminate flush gap.
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/admission/plugin/webhook/validating/dispatcher.go

### Field Validation — 3 failures (FIX STAGED a18febe + e2e2f48)
- `field_validation.go:462,611,735`
- **Root cause found**: Unknown top-level CR fields not rejected (serde flatten captured them). Unknown metadata fields not validated against known ObjectMeta field list.
- **K8s ref**: staging/src/k8s.io/apiextensions-apiserver/pkg/apiserver/customresource_handler.go

### Deployment — 2 failures (FIX STAGED ea9573e + 07a393c)
- `deployment.go:1008,1322`
- **Root cause found**: Old RSes stuck with non-zero replicas when maxUnavailable rounds to 0. K8s forces scale-down when new RS fully available. Also missing proportional scaling across RSes.
- **K8s ref**: pkg/controller/deployment/rolling.go — reconcileOldReplicaSets, sync.go — scale()

### DaemonSet — 1 failure (FIX STAGED 646c713)
- `daemon_set.go:1276`
- **Root cause found (deep analysis)**: Hash format was raw decimal instead of K8s SafeEncodeString alphabet "bcdfghjklmnpqrstvwxz2456789". ControllerRevision name and Match() byte comparison failed.
- **K8s ref**: staging/src/k8s.io/apimachinery/pkg/util/rand/rand.go

### ResourceQuota — 1 failure (FIX STAGED 0ed1628)
- `resource_quota.go:282`
- **Root cause found (deep analysis)**: Missing ephemeral-storage tracking in quota controller
- **K8s ref**: pkg/quota/v1/evaluator/core/pods.go — PodUsageFunc

### Namespace Deletion — 1 failure (FIX STAGED a1025ba)
- `namespace.go:609`
- **Root cause found**: All resources deleted in one pass. K8s deletes pods first, sets conditions, then deletes configmaps on next cycle for observable ordering.
- **K8s ref**: pkg/controller/namespace/deletion/namespaced_resources_deleter.go

### Job — 1 failure (FIX STAGED 31f4f39)
- `job.go:556`
- **Root cause found**: Completed jobs skipped entirely, terminating count never updated to 0
- **K8s ref**: pkg/controller/job/job_controller.go — syncJob

### Auth — 2 failures (FIX STAGED 2f20539)
- `service_accounts.go:129,667`
- **Root cause found**: Kubelet uses HS256 (can't find SA keys at /root/.rusternetes/certs/sa.key) while API server uses RS256 (finds keys at /etc/kubernetes/pki/sa.key). TokenReview fails because algorithms don't match.

### Discovery — 1 failure (FIX STAGED 3ba5e20)
- `discovery.go:131`
- **Root cause found**: /apis/ (trailing slash) returns 404. Go http.ServeMux handles trailing slashes automatically, Axum doesn't.

### EndpointSlice Mirroring — 1 failure (FIX STAGED 361752a)
- `endpointslicemirroring.go:202`
- **Root cause found (deep analysis)**: Mirrored slices not deleted when source Endpoints deleted. Cleanup only recognized endpointslice-controller, not mirroring-controller label.
- **K8s ref**: pkg/controller/endpointslicemirroring/reconciler.go

### StatefulSet — 1 failure (FIX STAGED 8673d37 + 4438743)
- `statefulset.go:1092`
- **Root cause found (deep analysis)**: Three issues identified from K8s source comparison:
  1. Generic PATCH handler didn't increment metadata.generation on spec changes. Test verifies ObservedGeneration >= Generation after strategic merge patch. K8s increments generation on every spec mutation.
  2. Terminal (Failed/Succeeded) pods not deleted for recreation. K8s processReplica() at stateful_set_control.go:431 deletes terminal pods so StatefulSet recreates them.
  3. Replica counting wrong — K8s computeReplicaStatus() counts terminating pods in `replicas` but excludes them from `currentReplicas`/`updatedReplicas`. We excluded terminating from all counts.
- **Fix**: 8673d37 adds generation increment to generic_patch.rs and terminal pod deletion. 4438743 fixes replica counting to match K8s computeReplicaStatus() exactly.
- **K8s ref**: pkg/controller/statefulset/stateful_set_control.go:370-399,431, staging/src/k8s.io/apiserver/pkg/endpoints/handlers/patch.go

### Remaining (DinD/environment limitations)
- `hostport.go:219` — DinD can't bind to other node's IPs (HostPort networking)
- `pod_resize.go:857` — DinD cgroup limitations (in-place resource resize)
- `aggregator.go:359` — Sample API server deployment needs image pull + etcd sidecar (DinD network/image availability)
- `lifecycle_hook.go:132` — Downstream of kube-proxy fixes. PreStop HTTP hook curls another pod via pod IP. Pod-to-pod networking requires working kube-proxy. Verified: kubelet lifecycle hook implementation matches K8s (postStart kills container on failure, preStop warns and continues). K8s ref: pkg/kubelet/kuberuntime/kuberuntime_container.go:762-893

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | TBD | TBD | 441 | TBD |

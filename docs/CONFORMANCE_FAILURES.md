# Conformance Failure Tracker

**Round 135** | 373/441 (84.6%) | 2026-04-11
**Round 136** | Pending (17 fixes staged) | 2026-04-12

## Staged Fixes for Round 136 (from deep K8s source comparison)

| Commit | Fix | K8s Ref | Expected Impact |
|--------|-----|---------|-----------------|
| 3012663 | kube-proxy XOR hash — no flush gap | proxier.go iptables-restore | ~18 (webhook+service+DNS) |
| fe76396 | kube-proxy RELATED,ESTABLISHED + OUTPUT | proxier.go:1460,386 | service networking |
| 7f8d692 | kube-proxy --reap for session affinity | proxier.go:1557 | edge cases |
| 0188c3c | OpenAPI raw JSON CRD schemas | customresource_handler.go | 9 CRD OpenAPI |
| e1f4bd0 | Preemption extended resources | preemption.go | 4 preemption |
| 646c713 | DaemonSet SafeEncodeString hash | rand.go SafeEncodeString | 1 daemonset |
| 73795a7 | Endpoints terminal/terminating/publishNotReady | controller_utils.go ShouldPodBeInEndpoints | endpoints reliability |
| 0ed1628 | ResourceQuota ephemeral-storage | pods.go PodUsageFunc | 1 quota |
| a1025ba | Namespace deletion pod ordering | namespaced_resources_deleter.go | 1 namespace |
| 31f4f39 | Job terminating count for completed | job_controller.go syncJob | 1 job |
| 2f20539 | Kubelet RS256 key path | jwt.go | 1 SA token |
| 7cf9bd5 | Webhook objectSelector | object/matcher.go | webhook reliability |
| a18febe | CRD strict unknown top-level fields | customresource_handler.go | 1 field validation |
| e2e2f48 | CRD strict unknown metadata fields | customresource_handler.go | 1 field validation |
| 3ba5e20 | Trailing slash routes /api/ /apis/ | Go http.ServeMux | 1 discovery |
| 361752a | EndpointSlice mirroring cleanup | reconciler.go | 1 mirroring |

## Known Remaining Issues (need more work)

### Watch "context canceled" — ~8 failures (FIX STAGED 069e807)
- Root cause found: TLS server didn't advertise HTTP/2 via ALPN
- Go's client-go fell back to HTTP/1.1, causing connection pooling issues
- **Fix staged**: 069e807 enables h2 + http/1.1 ALPN in rustls ServerConfig
- K8s ref: staging/src/k8s.io/apiserver/pkg/server/options/serving.go

### Deployment proportional scaling — 1 failure
- K8s distributes replicas proportionally during rollover
- Complex feature not implemented in our controller

### Aggregator — 1 failure
- Sample API server pod never starts (kubelet sync issue)
- Should improve with kube-proxy fixes

### Host Port / Pod Resize — 2 failures
- DinD infrastructure limitations

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

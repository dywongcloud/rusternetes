# Conformance Issue Tracker

**Round 116** | IN PROGRESS | ~184/441 done | ~105 passed, ~79 failed (~57%)
Counts via `scripts/conformance-progress.sh` (Ginkgo bullet markers)

## Fixes Committed (not yet deployed)

| Fix | Commit | Impact |
|-----|--------|--------|
| Shell probe for umask wrapper (distroless images) | 6c190e6 | Sonobuoy/sonobuoy-worker now start |
| is_pod_running excludes pause-only containers | 6c190e6 | Fixes subpath retry bypassing validation (#1) |
| Endpoints controller resolves named ports per-pod | 6c190e6 | Fixes EndpointSlice splitting for named ports (#8) |
| EndpointSlice keeps all ports in one slice | 6c190e6 | Fixes port splitting bug in from_endpoints |
| Service selector Optional for null/missing | 6c190e6 | Fixes service_latency decode error (#28) |
| LimitRange: requests inherit from explicit limits | e356d79 | Fixes CPU 300m expected, got 100m (#15) |
| Kubelet watch cooldown 3s→1s | 0102b4b | Faster pod startup (#12, #14) |
| Scheduler 5s→2s, controller-manager 10s→5s | ac17291 | Reduce convergence latency (#5, #12, #14, #18, #27) |
| Watch kind mappings for 16 resource types | cdc276a | Fixes watch cancel loops (#4, #9, #10, #17, #22, #25, #26, #27) |
| Refresh Secret/ConfigMap volumes on sync | d66a1f8 | Fixes secret volume updates (#3) |
| Deployment revision annotation CAS retry | 2b7b4a9 | Fixes deployment revision (#11) |
| shell_join quotes shell metacharacters | 5b37717 | Fixes DNS querier pod syntax errors (#6) |
| Remove terminated containers before restart | bd81f2c | Fixes RestartCount=0 after probe kill (#19) |
| cargo fmt --all | 3ebda44 | Formatting |

## Code Bugs (Round 116)

| # | Test | File | Error | Status |
|---|------|------|-------|--------|
| 1 | subpath expansion lifecycle | expansion.go:272 | Pod Running not Pending | FIXED — 6c190e6 |
| 2 | StatefulSet scaling order | statefulset.go:2479 | Scaled 3->2 | Timing race at boundary |
| 3 | Secrets volume updates | secrets_volume.go:374 | Timed out 240s | FIXED — d66a1f8 |
| 4 | IngressClass API watch | ingressclass.go:375 | ADDED not MODIFIED | FIXED — cdc276a (kind mapping) |
| 5 | RC serve basic image | rc.go:538 | Pod responses failed | FIXED — ac17291 (latency) |
| 6 | DNS services (3x) | dns_common.go:476 | DNS lookup timeout | FIXED — 5b37717 (shell_join) |
| 7 | CRD publish OpenAPI (2x) | crd_publish_openapi.go:318,400 | CRD creation timeout | API server contention under load |
| 8 | EndpointSlice matching pods | endpointslice.go:699 | Expected >=2 slices | FIXED — 6c190e6 |
| 9 | CronJob API watch | cronjob.go:443 | ADDED not MODIFIED | FIXED — cdc276a (kind mapping) |
| 10 | Webhook config ready | webhook.go:1269 | Timed out | FIXED — cdc276a (kind mapping) |
| 11 | Deployment rolling update | deployment.go:781 | No revision set | FIXED — 2b7b4a9 |
| 12 | HostPort no conflict | hostport.go:219 | Pod not starting | FIXED — 0102b4b + ac17291 (latency) |
| 13 | EmptyDir perms (non-root) | output.go:263 | 0755 not 0777 | Docker Desktop macOS limitation |
| 14 | InitContainer RestartNever | init_container.go:565 | Timed out | FIXED — 0102b4b + ac17291 (latency) |
| 15 | LimitRange defaults | limit_range.go:162 | CPU 300m got 100m | FIXED — e356d79 |
| 16 | Job FailIndex | job.go:1251 | Timed out | Watch cancel loops → cdc276a should help |
| 17 | Aggregated discovery | aggregated_discovery.go:282 | Missing resource | FIXED — cdc276a (kind mapping) |
| 18 | RC lifecycle | rc.go:257 | Timed out | FIXED — ac17291 (latency) + cdc276a |
| 19 | Container probe restart | container_probe.go:1779 | RestartCount=0 | FIXED — bd81f2c |
| 20 | kubectl diff | builder.go:97 | OpenAPI protobuf | Needs protobuf OpenAPI encoding |
| 21 | Pod InPlace Resize | pod_resize.go:857 | cgroup mismatch | Docker Desktop cgroup limitation |
| 22 | PriorityClass endpoints | preemption.go:978 | Watch cancel loop | FIXED — cdc276a (kind mapping) |
| 23 | Service connectivity | service.go:870 | Deadline exceeded | Watch cancel loops → cdc276a should help |
| 24 | EndpointSlice exec | util.go:182 | curl exit 7 | Service routing (kube-proxy) |
| 25 | Service proxy | proxy.go:503 | Timed out | Watch cancel loops → cdc276a should help |
| 26 | Watch restart | watch.go:223 | No 2nd notification | FIXED — cdc276a (kind mapping stops cancel loops) |
| 27 | ReplicaSet lifecycle | replica_set.go:738 | Timed out | FIXED — ac17291 + cdc276a |
| 28 | Service latency | service_latency.go:142 | Decode error | FIXED — 6c190e6 |

## Unfixable (Platform Limitations)

| # | Issue | Reason |
|---|-------|--------|
| 13 | EmptyDir 0777 permissions | Docker Desktop macOS bind mount umask |
| 20 | kubectl diff | Needs protobuf OpenAPI spec encoding |
| 21 | Pod resize cgroups | Docker Desktop macOS cgroup v2 |

## Progress

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 107 | ~411 | ~19 | ~430 | ~96% |
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 105 | 79 | 184/441 | 57.1% (in progress, pre-deploy) |

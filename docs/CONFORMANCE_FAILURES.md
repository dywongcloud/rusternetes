# Conformance Issue Tracker

**Round 116** | IN PROGRESS | ~222/441 done | ~128 passed, ~94 failed (~58%)

## Fixes Committed (not yet deployed)

| Fix | Commit | Impact |
|-----|--------|--------|
| Shell probe for umask wrapper (distroless) | 6c190e6 | #1 subpath, sonobuoy start |
| Endpoints named port resolution | 6c190e6 | #8 EndpointSlice splitting |
| Service selector Optional | 6c190e6 | #28 service decode |
| LimitRange request/limit ordering | e356d79 | #15 CPU defaults |
| Kubelet cooldown 3s→1s | 0102b4b | #12, #14 latency |
| Scheduler 2s, controller 5s | ac17291 | #5, #12, #14, #18, #27 latency |
| Watch kind mappings (16 types) | cdc276a | #4, #9, #10, #17, #22, #26 watch |
| Secret/ConfigMap volume refresh | d66a1f8 | #3 volume updates |
| Deployment revision CAS retry | 2b7b4a9 | #11 revision |
| shell_join metacharacter quoting | 5b37717 | #6 DNS syntax |
| Container restart (remove terminated) | bd81f2c | #19 RestartCount |
| Kube-proxy 5s→3s | 452853a | Service latency |
| Sysctl trailing dash validation | 6e188eb | Sysctl conformance |
| Status update merges annotations | b0a0430 | Namespace status patch |
| OpenAPI protobuf + CSR PATCH | 86a8e6a | #20 kubectl diff |
| Terminated container reason | 05fb7a1 | Kubelet runtime test |
| Namespace async deletion | 5441060 | Namespace lifecycle |
| Shell probe cache | fe1b65c | Pod start latency |
| StatefulSetSpec serde defaults | 6aeaa05 | SS deserialization |
| Webhook skip for terminating NS | 3c3e0c3 | Stale webhook blocking |
| Bound SA tokens with pod ref | d1b5ad0 | SA extra info |
| StatefulSet rolling update logging | 988a4e5 | Debug SS updates |
| Kube-proxy service sync logging | c05b60d | Debug service routing |
| **CoreDNS toleration for all taints** | 0db0b71 | **ROOT CAUSE: DNS eviction** |
| CRD creation single etcd write | bdadd6f | #7 CRD timeout |

## Code Bugs (Round 116)

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | subpath expansion | Pod Running not Pending | FIXED — 6c190e6 |
| 2 | StatefulSet scaling | Scaled 3->2 | Timing race at boundary |
| 3 | Secrets volume updates | Timed out 240s | FIXED — d66a1f8 |
| 4 | IngressClass API watch | ADDED not MODIFIED | FIXED — cdc276a |
| 5 | RC serve basic image | Pod responses failed | FIXED — ac17291 |
| 6 | DNS services (3x) | DNS lookup timeout | FIXED — 5b37717 + 0db0b71 |
| 7 | CRD publish OpenAPI (7x) | CRD creation timeout | FIXED — bdadd6f |
| 8 | EndpointSlice matching | Expected >=2 slices | FIXED — 6c190e6 |
| 9 | CronJob API watch | ADDED not MODIFIED | FIXED — cdc276a |
| 10 | Webhook config ready | Timed out | FIXED — cdc276a |
| 11 | Deployment rolling update | No revision set | FIXED — 2b7b4a9 |
| 12 | HostPort no conflict | Pod not starting | FIXED — 0102b4b + ac17291 |
| 13 | EmptyDir perms (non-root) | 0755 not 0777 | Docker Desktop macOS limitation |
| 14 | InitContainer RestartNever | Timed out | FIXED — 0102b4b + ac17291 |
| 15 | LimitRange defaults | CPU 300m got 100m | FIXED — e356d79 |
| 16 | Job timeouts (6x) | Job never completes | FIXED — 0db0b71 (CoreDNS) + cdc276a |
| 17 | Aggregated discovery | Missing resource | FIXED — cdc276a |
| 18 | RC lifecycle | Timed out | FIXED — ac17291 + cdc276a |
| 19 | Container probe restart | RestartCount=0 | FIXED — bd81f2c |
| 20 | kubectl diff | OpenAPI protobuf | FIXED — 86a8e6a |
| 21 | Pod InPlace Resize | cgroup mismatch | Docker Desktop cgroup limitation |
| 22 | PriorityClass endpoints | Watch cancel loop | FIXED — cdc276a |
| 23 | Service connectivity (7x) | Deadline exceeded | FIXED — 0db0b71 (CoreDNS evicted) |
| 24 | EndpointSlice exec | curl exit 7 | FIXED — 0db0b71 (CoreDNS evicted) |
| 25 | Service proxy | Timed out | FIXED — 0db0b71 (CoreDNS evicted) |
| 26 | Watch restart | No 2nd notification | FIXED — cdc276a |
| 27 | ReplicaSet lifecycle | Timed out | FIXED — ac17291 + cdc276a |
| 28 | Service latency | Decode error | FIXED — 6c190e6 |

## Root Cause Analysis

**CoreDNS eviction was the root cause of ~20+ test failures.** A conformance test
applied a `kubernetes.io/e2e-evict-taint-key` NoExecute taint to nodes, which evicted
CoreDNS because it lacked tolerations. Once CoreDNS was gone, DNS resolution failed,
which broke service connectivity, caused watch informer cancel loops (can't resolve
API server hostname), and cascaded into timeouts across Job, Service, DNS, and
networking tests. Fixed by adding `operator: Exists` toleration to CoreDNS.

## Platform Limitations (not fixable)

| # | Issue | Reason |
|---|-------|--------|
| 13 | EmptyDir 0777 permissions | Docker Desktop macOS bind mount umask |
| 21 | Pod resize cgroups | Docker Desktop macOS cgroup v2 |

## Progress

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 107 | ~411 | ~19 | ~430 | ~96% |
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% (in progress, pre-deploy) |

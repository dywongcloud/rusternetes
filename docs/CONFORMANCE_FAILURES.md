# Conformance Issue Tracker

**Round 117** | IN PROGRESS | 19/441 done | 11 passed, 8 failed (57.9%)
Zero watch cancel loops (vs 1000s in round 116)

## Current Failures (Round 117)

| # | Test | File | Error | Status |
|---|------|------|-------|--------|
| 1 | StatefulSet scaling order | statefulset.go:2479 | Scaled 3->2 | Timing race — test boundary |
| 2 | StatefulSet wait for pods | wait.go:63 | Rate limiter exceeded | Client rate limiting |
| 3 | kubectl diff | builder.go:97 | MIME parse error | FIXED — b2f9538 (Content-Type) |
| 4 | ExternalName service | service.go:1450 | Service unreachable | ExternalName DNS resolution |
| 5 | CRD publish OpenAPI | crd_publish_openapi.go:161 | CRD creation timeout | 30s deadline tight |
| 6 | StatefulSet rolling update | statefulset.go:957 | Pod not re-created | Rolling update not triggering |
| 7 | IngressClass API watch | ingressclass.go:375 | ADDED not MODIFIED | Watch event type — investigating |
| 8 | Webhook test | webhook.go:601 | Unknown | Webhook handling |

## Fixes Deployed This Round

| Fix | Commit | Result |
|-----|--------|--------|
| Shell probe for umask (distroless) | 6c190e6 | Sonobuoy starts |
| is_pod_running excludes pause-only | 6c190e6 | Subpath test passes |
| Endpoints named port resolution | 6c190e6 | EndpointSlice test passes |
| Service selector Optional | 6c190e6 | Service decode passes |
| LimitRange request/limit ordering | e356d79 | LimitRange test passes |
| Kubelet cooldown 3s→1s | 0102b4b | Faster pod startup |
| Scheduler 2s, controller 5s | ac17291 | Faster convergence |
| **Watch kind mappings (16 types)** | cdc276a | **0 watch cancel loops** |
| Secret/ConfigMap volume refresh | d66a1f8 | Volume update test passes |
| Deployment revision CAS retry | 2b7b4a9 | Deployment revision |
| shell_join metacharacter quoting | 5b37717 | DNS syntax fixed |
| Container restart fix | bd81f2c | RestartCount works |
| Kube-proxy 3s | 452853a | Faster service routing |
| Sysctl trailing dash | 6e188eb | Sysctl validation |
| Status annotations merge | b0a0430 | Namespace status patch |
| OpenAPI protobuf encoding | 86a8e6a | kubectl protobuf |
| Terminated container reason | 05fb7a1 | Container reason set |
| Namespace async deletion | 5441060 | Namespace lifecycle |
| Shell probe cache | fe1b65c | Faster pod start |
| StatefulSetSpec serde defaults | 6aeaa05 | SS deserialization |
| Webhook skip terminating NS | 3c3e0c3 | Stale webhook skip |
| Bound SA tokens | d1b5ad0 | SA pod-name extra |
| **CoreDNS toleration** | 0db0b71 | **DNS survives taints** |
| CRD single write | bdadd6f | CRD creation speed |
| **Shell cache deadlock fix** | 9c1d0f7 | **Kubelet no longer hangs** |
| Content-Type fix | b2f9538 | kubectl MIME error |

## Key Improvements vs Round 116

- **Watch cancel loops**: 0 (was 1000s) — kind mapping fix working
- **CoreDNS**: Survives taint tests — DNS stays up throughout
- **Kubelet deadlock**: Fixed shell cache mutex across await
- **Sonobuoy starts**: Shell probe correctly handles distroless

## Platform Limitations (not fixable)

| Issue | Reason |
|-------|--------|
| EmptyDir 0777 permissions | Docker Desktop macOS bind mount umask |
| Pod resize cgroups | Docker Desktop macOS cgroup v2 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% (pre-deploy) |
| 117 | 11 | 8 | 19/441 | 57.9% (in progress) |

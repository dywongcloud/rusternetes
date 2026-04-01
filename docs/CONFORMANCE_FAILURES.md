# Conformance Issue Tracker

**Round 117** | IN PROGRESS | 96/441 done | 64 passed, 32 failed (66.7%)
Zero watch cancel loops. CoreDNS surviving taints.

## Current Failures (Round 117)

| # | Test | File | Error | Status |
|---|------|------|-------|--------|
| 1 | StatefulSet scaling | statefulset.go:2479 | Scaled 3->2 | Timing race |
| 2 | StatefulSet wait | wait.go:63 | Rate limiter | Pod startup latency |
| 3 | kubectl diff | builder.go:97 | MIME error | FIXED — b2f9538 Content-Type |
| 4 | ExternalName service | service.go:1450 | Unreachable | ExternalName DNS/routing |
| 5 | CRD publish OpenAPI (2x) | crd_publish_openapi.go:161,77 | CRD timeout | FIXED — 213585c async status update |
| 6 | StatefulSet rolling update | statefulset.go:957 | Not re-created | Rolling update detection |
| 7 | IngressClass watch | ingressclass.go:375 | ADDED not MODIFIED | **FIXED — ce2f9d3 label selector bug** |
| 8 | Webhook ready (2x) | webhook.go:601,1631 | Timed out | Webhook service reachability |
| 9 | Job adopt/release | job.go:974 | Pod not released | Job controller orphan handling |
| 10 | FlowSchema API | flowcontrol.go:661 | Unknown | FlowSchema API test |
| 11 | Field validation | field_validation.go:105 | Wrong format | FIXED — c182bfd duplicate vs unknown |
| 12 | HostPort | hostport.go:219 | Pod not starting | Latency |
| 13 | RC condition | rc.go:623 | Replicas unavailable | Pod startup latency |
| 14 | VAP watch | validatingadmissionpolicy.go:814 | ADDED not MODIFIED | **FIXED — ce2f9d3 label selector bug** |
| 15 | Ingress watch | ingress.go:232 | ADDED not MODIFIED | **FIXED — ce2f9d3 label selector bug** |
| 16 | PDB | disruption.go:372 | Unknown | PDB test |
| 17 | RC serve image | rc.go:538 | Unknown | RC test |
| 18 | PreStop hook | pre_stop.go:153 | Timed out | Networking |
| 19 | Watch restart | watch.go:223 | No 2nd notification | Watch delivery |
| 20 | Service affinity | service.go:4291 | Unreachable | Service routing |

## Fixes Not Yet Deployed (committed after round 117 started)

| Fix | Commit | Expected Impact |
|-----|--------|----------------|
| **Watch MODIFIED→ADDED bug** | ce2f9d3 | **Fixes #7, #14, #15 + CronJob, FlowSchema** |
| Field validation format | c182bfd | Fixes #11 |
| CRD async status update | 213585c | Fixes #5 |
| Content-Type fix | b2f9538 | Fixes #3 |
| Watch parameter logging | dd468e2 | Diagnostics |
| Shell cache deadlock | 9c1d0f7 | Already deployed — kubelet works |

## Deployed Fixes (this round)

| Fix | Commit | Result |
|-----|--------|--------|
| Watch kind mappings (16 types) | cdc276a | **0 watch cancel loops** |
| CoreDNS toleration | 0db0b71 | **DNS survives taints** |
| Shell cache deadlock | 9c1d0f7 | **Kubelet no longer hangs** |
| Shell probe distroless | 6c190e6 | Sonobuoy starts |
| Subpath retry fix | 6c190e6 | Subpath test passes |
| Named port resolution | 6c190e6 | EndpointSlice passes |
| Service selector Optional | 6c190e6 | Service decode passes |
| LimitRange ordering | e356d79 | LimitRange passes |
| Interval reductions | 0102b4b, ac17291, 452853a | Faster convergence |
| Secret/ConfigMap refresh | d66a1f8 | Volume updates pass |
| Deployment revision CAS | 2b7b4a9 | Revision set |
| shell_join quoting | 5b37717 | DNS syntax |
| Container restart | bd81f2c | RestartCount works |
| Sysctl validation | 6e188eb | Sysctl passes |
| Status annotations | b0a0430 | NS status patch |
| OpenAPI protobuf | 86a8e6a | kubectl protobuf |
| Terminated reason | 05fb7a1 | Container reason |
| Namespace async delete | 5441060 | NS lifecycle |
| Webhook skip terminating | 3c3e0c3 | Stale webhooks |
| Bound SA tokens | d1b5ad0 | SA pod-name extra |
| CRD single write | bdadd6f | CRD speed |
| StatefulSetSpec defaults | 6aeaa05 | SS deserialization |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% (pre-deploy) |
| 117 | 42 | 23 | 65/441 | 64.6% (in progress) |

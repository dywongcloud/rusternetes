# Conformance Issue Tracker

**Round 114** | IN PROGRESS | 24/441 done | 16 passed, 8 failed (66.7%)

## Round 114 Failures

### Code Bugs (2)
| Test | Error | Status |
|------|-------|--------|
| StatefulSet Scaling predictable order | "scaled 3 -> 0 replicas" | Investigating — all pods deleted during rolling update |
| Service ExternalName → NodePort | "not reachable within 2m0s" | Service type change not routing |

### Timeouts (6)
| Test | Error |
|------|-------|
| kube-root-ca.crt in namespace | timed out |
| CRD FieldValidation invalid CR | CRD creation timeout |
| CRD multiple CRDs same group | CRD creation timeout |
| CRD preserving unknown fields | CRD creation timeout |
| Proxy pod/service responses | Pod didn't start in time |
| EndpointSlice for matching Pods | EndpointSlice creation timeout |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 110 | 158 | 441 | 64.2% |
| 114 | 8 | 24/441 | 66.7% (in progress) |

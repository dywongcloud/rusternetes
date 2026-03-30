# Conformance Issue Tracker

**Round 114** | IN PROGRESS | 47/441 done | 28 passed, 19 failed (59.6%)

## Code Bugs (6)
| Test | Error | Status |
|------|-------|--------|
| StatefulSet Scaling | "scaled 3 -> 0" | FIXED (6d625c9) — not deployed yet |
| IngressClass API | ADDED instead of MODIFIED | Watch event type — reconnect sends ADDED |
| Deployment proportional scaling | RS never reached availableReplicas | Controller timing |
| Service ExternalName→NodePort | "not reachable" | Endpoint/kube-proxy sync timing |
| Pod InPlace Resize | Resize state verification | Complex resize feature |
| SA OIDC discovery | Pod failed to start | Pod startup timeout |

## Timeouts (13)
CRD creation (3), kube-root-ca.crt, Proxy pod start, EndpointSlice, Preemption, and others — Docker Desktop latency.

## Pending Fixes (not deployed)
| Fix | Commit | Impact |
|-----|--------|--------|
| StatefulSet scale down one-at-a-time | 6d625c9 | Fixes "3 -> 0" |
| Terminal pod cleanup 5-min delay | 7676966 | Fixes fake logs (8 tests) |
| virtiofs sync after volumes | 00fafbb | Fixes volume content empty |
| DaemonSet deterministic hash | fb12f97 | Fixes pod thrashing |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 110 | 158 | 441 | 64.2% |
| 114 | 19 | 47/441 | 59.6% (in progress) |

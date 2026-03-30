# Conformance Issue Tracker

**Round 113** | IN PROGRESS | All fixes deployed from clean start

## Deployed Fixes (85 total)

All previous conformance fixes deployed — see git log for full list. Key fixes this cycle:
- 80 non-timeout fixes from Round 110 analysis
- DaemonSet deterministic pod names (f2e521d)
- DaemonSet deterministic template hash (fb12f97) — fixes pod thrashing
- Orphan cleanup 60s grace period + all-pod check (2b82b7f)
- Terminal pod container cleanup (044767d)
- fsGroup g+rwX permissions (cc2f8b8)
- ConfigMap/Secret volume resync items fix (de78bc8)
- EmptyDir bind mount revert (add9c7d)
- kube-proxy sync 5s + session affinity DNAT fixes (a37b4c3 + 80d5fb8)

## Round 113 Failures (36/441 = 55.6%, 20 passed, 16 failed)

### Timeouts (6)
- StatefulSet scaling, InitContainer RestartAlways, Webhook timeout, RC scale, Preemption disruption, Service ClusterIP→ExternalName

### Code Bugs Found and Fixed
| Bug | Fix | Commit |
|-----|-----|--------|
| Pod logs returning synthetic fake output | Delay container cleanup 5 min | 7676966 |
| Volume files invisible in containers (virtiofs cache) | sync() after volume creation | 00fafbb |
| DaemonSet template hash non-deterministic | Value normalization | fb12f97 |

The fake logs fix addresses 8 failures: ConfigMap/Secret/Projected volume content, env vars, file modes, Downward API modes — all were returning "Container starting" fake output instead of real mounttest output.

### Remaining Code Bugs
| Bug | Error |
|-----|-------|
| Pod InPlace Resize | Resize state verification (complex) |
| PriorityClass endpoints | Value mismatch (stale cluster data) |
| Deployment proportional scaling | RS never reached replicas |
| StatefulSet rolling updates | Revision tracking |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |
| 113 | ? | 0/441 | IN PROGRESS |

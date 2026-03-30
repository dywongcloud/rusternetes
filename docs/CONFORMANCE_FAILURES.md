# Conformance Issue Tracker

**Round 115** | IN PROGRESS | All fixes deployed from clean start

## Deployed Fixes (90+ total)

Key fixes in this round:
- Terminal pod container cleanup delayed 5 min (7676966) — fixes fake log output
- virtiofs sync after volume creation (00fafbb) — fixes empty volume content
- DaemonSet deterministic template hash (fb12f97) — fixes pod thrashing
- DaemonSet deterministic pod names (f2e521d) — stable pod names per node
- StatefulSet scale down one-at-a-time (6d625c9) — fixes "3→0" scaling
- Orphan cleanup 60s grace + all-pod check (2b82b7f) — prevents cross-node kills
- fsGroup g+rwX permissions (cc2f8b8) — proper group-write on volume files
- ConfigMap/Secret volume resync items fix (de78bc8)
- Plus all 80+ fixes from earlier rounds

## Round 115 Failures (21/441 = 76.2% early)

### Code Bugs (1)
| Test | Error |
|------|-------|
| EndpointSlice API operations | Expected (need to check specific assertion) |

### Timeouts (4)
| Test | Error |
|------|-------|
| StatefulSet Scaling | Pods didn't enter running (timeout, NOT "3→0" — scale-down fix working!) |
| Service NodePort→ExternalName | context deadline exceeded |
| Endpoint lifecycle | MODIFIED event not seen |
| Service multiport endpoints | context deadline exceeded |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 110 | 158 | 441 | 64.2% |
| 114 | ~21 | ~51/441 | ~59% (incomplete) |
| 115 | 5 | 21/441 | 76.2% (early — 1 bug, 4 timeouts) |

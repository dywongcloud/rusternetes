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

## Round 115 Failures

(monitoring — tests initializing)

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 110 | 158 | 441 | 64.2% |
| 114 | ~21 | ~51/441 | ~59% (incomplete) |
| 115 | ? | 0/441 | IN PROGRESS |

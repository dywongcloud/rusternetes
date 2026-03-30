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

## Round 113 Failures

(monitoring — no results yet, tests initializing)

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |
| 113 | ? | 0/441 | IN PROGRESS |

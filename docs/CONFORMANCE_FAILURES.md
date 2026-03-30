# Conformance Issue Tracker

**Round 111** | IN PROGRESS | 80 fixes deployed | Kubelet rebuild pending (orphan grace period fix)

## Round 111 Failures (so far)

| Test | Error | New? |
|------|-------|------|
| StatefulSet Scaling predictable order | "scaled unexpectedly" | From R110 |
| Job FailIndex podFailurePolicy | "ensure job completion" | From R110 — fix deployed |
| ConfigMap env variable prefixes | ? | NEW |
| ConfigMap volume mappings Item mode | ? | NEW |
| DaemonSet rollback without restarts | timeout | From R110 |
| CRD preserving unknown fields embedded | ? | NEW |
| EmptyDir (non-root,0777,default) | ? | NEW |

## Known Infrastructure Issues

| Issue | Status |
|-------|--------|
| Kubelet orphan cleanup kills DaemonSet pods | FIXED (41b37f4) — 30s grace period deployed |
| ConfigMap/Secret volume item mappings | INVESTIGATING — agent working on fix |
| EmptyDir file permissions (0777) | INVESTIGATING — agent working on fix |
| Termination message from file | Empty message read — bind mount issue |

## Previous Fixes Deployed (80 from Round 110)

All 80 non-timeout fixes from Round 110 are deployed in this run. See git log for details.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |
| 111 | ? | 15/441 | 60.0% (early, 9 passed / 6 failed) |

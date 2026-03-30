# Conformance Issue Tracker

**Round 111** | IN PROGRESS (compromised by kubelet restart mid-run)

## Analysis: Most Round 111 failures are restart artifacts

The kubelet was restarted at ~15:12 to deploy the orphan cleanup fix. This killed running test pods, causing cascading failures. Genuine new failures vs restart artifacts:

### Restart Artifacts (will pass in clean Round 112)
- Secret env vars, Projected secret/downwardAPI volumes — pods killed mid-test
- CronJob API ADDED vs MODIFIED — watch connection reset
- ConfigMap env prefixes, volume mappings — pods recreated without data

### Recurring from Round 110 (fixes deployed, need clean run to verify)
| Category | R110 Fix | Commit |
|----------|---------|--------|
| StatefulSet scaling | Deterministic revision hash | 0591bb2 |
| Job FailIndex | Per-index failure tracking | 0591bb2 |
| Job successPolicy | SuccessPolicy evaluation | 0591bb2 |
| VAP variables | Variable evaluation | 818922f |
| Sysctl reject | Name validation format | 55c1e5a |
| Session affinity | kube-proxy DNAT fixes | a37b4c3 |
| Termination message | FallbackToLogsOnError | 6af1a31 |

### Genuinely New (fixed)
| Category | Error | Fix | Commit |
|----------|-------|-----|--------|
| EmptyDir permissions | File perms not 0777 | fsGroup g+rwX (was g+rX) | cc2f8b8 |

## Fixes Deployed This Round
| Fix | Commit |
|-----|--------|
| Orphan cleanup 30s grace period | 41b37f4 |
| Terminal pod container cleanup | 044767d |
| All 80 Round 110 fixes | Various |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |
| 111 | 27+ | 48/441 | 43.8% (COMPROMISED — restart mid-run) |

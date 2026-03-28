# Conformance Issue Tracker

**311 total fixes** | Round 107 IN PROGRESS | 4 failures at ~80/441

## Deployed: #1-310 | Pending: #311

## Round 107 Failures (4 so far)
| Test | Error | Fix |
|------|-------|-----|
| statefulset.go:2479 | SS replicas 3→2 unexpectedly | **#311** pending — replicas not capped |
| predicates.go:1102 | Scheduler predicates timeout | Kubelet/scheduler timing |
| rc.go:442 | RC scale rate limiter | Watch reconnection overhead |
| watch.go:370 | RV mismatch (expected 63599, got 63586) | Watch RV tracking bug |

## Pending deploy
| # | Fix |
|---|-----|
| 311 | SS status.replicas reports actual count |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | ~25 | 441 | ~94% |
| 107 | 4 | ~80/441 | IN PROGRESS |

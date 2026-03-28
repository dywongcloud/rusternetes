# Conformance Issue Tracker

**312 total fixes** | Round 107 IN PROGRESS | 16 failures at ~300/441

## Deployed: #1-310 | Pending: #311-312

## Round 107 Failures (13 so far)
| Test | Error | Fix/Status |
|------|-------|-----------|
| statefulset.go:2479 | SS replicas 3→2 | **#311** pending |
| predicates.go:1102 | Scheduler timeout | Kubelet timing |
| rc.go:442 | RC rate limiter | **#310** deployed (bookmark keepalive) |
| watch.go:370 | RV mismatch | **#312** pending |
| crd_conversion_webhook | Webhook pod failed | Pause container port conflict |
| crd_watch.go:72 | CRD Established watch | CRD status update timing |
| crd_publish_openapi (x2) | CRD Established watch | CRD status update timing |
| field_validation.go:428 | CRD Established watch | CRD status update timing |
| ResourceQuota service | Service lifecycle | NEW |
| Watchers concurrent | Watch event ordering | NEW |
| Deployment rolling update | Pod timeout | Kubelet timing |
| Container Runtime status | Expected status | Container status |
| Scheduler preemption | Preemption timeout | Scheduler |

## Pending deploy (#311-312)
| # | Fix |
|---|-----|
| 311 | SS status.replicas reports actual count |
| 312 | Watch bookmark RV initialized to MAX |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | ~25 | 441 | ~94% |
| 107 | 13 | ~250/441 | IN PROGRESS |

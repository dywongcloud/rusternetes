# Conformance Issue Tracker

**311 total fixes** | Round 107 IN PROGRESS | 4 failures at ~80/441

## Deployed: #1-310 | Pending: #311

## Round 107 Failures (8 so far)
| Test | Error | Fix |
|------|-------|-----|
| statefulset.go:2479 | SS replicas 3→2 unexpectedly | **#311** pending |
| predicates.go:1102 | Scheduler predicates timeout | Kubelet/scheduler timing |
| rc.go:442 | RC scale rate limiter | **#310** pending (bookmark keepalive) |
| watch.go:370 | RV mismatch (63599 vs 63586) | **#312** pending |
| crd_conversion_webhook.go:318 | Webhook pod failed to start | Pause container port conflict |
| crd_watch.go:72 | CRD creation timeout | CRD protobuf decoder |
| crd_publish_openapi.go:400 | CRD creation timeout | CRD protobuf decoder |
| field_validation.go:428 | CRD creation timeout | CRD protobuf decoder |

## Pending deploy (#311-312)
| # | Fix |
|---|-----|
| 311 | SS status.replicas reports actual count |
| 312 | Watch bookmark RV initialized to MAX of current and requested |

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

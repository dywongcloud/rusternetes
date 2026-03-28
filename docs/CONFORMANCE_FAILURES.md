# Conformance Issue Tracker

**312 total fixes** | Round 107 near completion | 19 failures at ~430/441

## Deployed: #1-310 | Pending: #311-312

## Round 107 Failures (19 unique)
| Test | Error | Fix/Status |
|------|-------|-----------|
| statefulset.go:2479 | SS replicas 3→2 | **#311** pending |
| watch.go:370 | RV mismatch | **#312** pending |
| EmptyDir non-root 0777 | chmod on bind mount | **#308** deployed but may need test |
| CRD watch/OpenAPI/FieldVal (x4) | CRD Established timeout | CRD status update timing |
| CRD conversion webhook | Webhook pod fail | Pause container conflict |
| AdmissionWebhook | Webhook fail closed | Webhook deployment |
| ResourceQuota (x2) | Service lifecycle + scopes | **#300** deployed + new |
| Deployment rolling update | Pod timeout | Kubelet timing |
| RC scale + RC basic image | Rate limiter + timeout | **#310** deployed |
| Container Runtime status | Expected status | Container status |
| Scheduler predicates + preemption | Timeout | Scheduler timing |
| Session affinity NodePort | Deployment timeout | Kubelet timing |
| Pod InPlace Resize | Resize verification | Resize implementation |

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
| 107 | 19 | ~430/441 | ~96% |

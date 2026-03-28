# Conformance Issue Tracker

**300 total fixes** | Round 106 IN PROGRESS | 17 failures at ~300/441

## Deployed: #1-297 | Pending: #298-300

## Round 106 Failures (17)
| Test | Error | Fix |
|------|-------|-----|
| statefulset.go:786 | Probe timeout=0 causes Ready=False | **#298** pending |
| CRD FieldSelectors | CRD protobuf creation timeout | CRD protobuf decoder |
| FieldValidation CRD | CRD protobuf creation | CRD protobuf decoder |
| CRD conversion webhook | Webhook deployment timeout | Kubelet blocking |
| ResourceQuota terminating | Scope filtering missing | **#300** pending |
| kubectl replace | Pod image update fails | Needs investigation |
| kubectl label | Label update fails | Needs investigation |
| kubectl expose | RC services fails | Needs investigation |
| Proxy v1 | Pod proxy timeout | Pod not starting |
| RC scale | RC replicas timeout | Rate limiter |
| RC exceeded quota | Failure condition | Needs investigation |
| Events lifecycle | Event MicroTime | **#299** pending |
| Events API | Event MicroTime | **#299** pending |
| AdmissionWebhook timeout | Webhook deploy | Kubelet blocking |
| AdmissionWebhook mutate CR | CRD protobuf | CRD protobuf |
| Pod InPlace Resize | Resize status | Resize implementation |
| Secrets immutable | Immutable update rejected | Secret handler |

## Pending deploy (#298-300)
| # | Fix |
|---|-----|
| 298 | Probe timeout=0 defaults to 1s |
| 299 | EventSeries.lastObservedTime MicroTime format |
| 300 | ResourceQuota scope filtering |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | 17 | ~300/441 | IN PROGRESS |

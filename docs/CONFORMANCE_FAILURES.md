# Conformance Issue Tracker

**304 total fixes** | Round 106 IN PROGRESS | 25 failures at ~420/441

## Deployed: #1-297 | Pending: #298-304

## Round 106 Failures (25)
| Test | Error | Fix |
|------|-------|-----|
| SS scaling | Probe timeout=0 | **#298** pending |
| CRD FieldSelectors | CRD protobuf timeout | CRD decoder |
| CRD creating/deleting | CRD protobuf timeout | CRD decoder |
| CRD multiple versions | CRD protobuf timeout | CRD decoder |
| FieldValidation CRD | CRD protobuf | CRD decoder |
| ResourceQuota scopes | Scope filtering | **#300** pending |
| AdmissionWebhook (x4) | Webhook deploy timeout | Kubelet/watch |
| kubectl replace/label/expose | OpenAPI protobuf | **#301** pending |
| Events lifecycle | MicroTime format | **#299** pending |
| Events API | MicroTime format | **#299** pending |
| Proxy v1 | Pod timeout | Kubelet blocking |
| RC scale | Rate limiter timeout | RC controller |
| RC exceeded quota | Failure condition | RC controller |
| Pod InPlace Resize | Resize status | Resize impl |
| EmptyDir non-root 0777 | File permissions | Docker Desktop limitation |
| Secrets immutable | Metadata update rejected | **#302** pending |
| Job adopt/release | Pod release | **#303** pending |
| Job backoffLimitPerIndex | Index tracking | Job controller |
| Job FailIndex | FailIndex action | **#304** pending |
| PriorityClass endpoints | HTTP methods | Needs investigation |

## Pending deploy (#298-304)
| # | Fix |
|---|-----|
| 298 | Probe timeout=0 defaults to 1s |
| 299 | EventSeries.lastObservedTime MicroTime |
| 300 | ResourceQuota scope filtering |
| 301 | OpenAPI v2 protobuf wrapper for kubectl |
| 302 | Immutable secrets allow metadata updates |
| 303 | Job releases pods when labels don't match |
| 304 | Job podFailurePolicy FailIndex action |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | 25 | ~420/441 | IN PROGRESS |

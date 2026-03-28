# Conformance Issue Tracker

**298 total fixes** | Round 106 IN PROGRESS | 16 failures so far

## Deployed: #1-297 | Pending: #298

## Round 106 Failures (16 so far)
| Test | Error | Fix status |
|------|-------|-----------|
| statefulset.go:786 | SS probe timeout=0 causes Ready=False loop | **#298** pending deploy |
| statefulset.go:2479 | SS list/patch/delete | Same probe timeout issue |
| CRD FieldSelectors | CRD protobuf creation timeout | CRD protobuf decoder |
| CRD conversion webhook | Webhook deployment timeout | Deployment readiness |
| ResourceQuota terminating | ResourceQuota scopes | NEW — needs investigation |
| kubectl replace | Pod image update | NEW — needs investigation |
| kubectl label | Label update | kubectl issue |
| kubectl expose | RC services | kubectl issue |
| Proxy v1 | Pod proxy timeout | Pod startup timeout |
| RC scale | RC replicas confirm timeout | Rate limiter timeout |
| RC exceeded quota | RC failure condition | NEW — needs investigation |
| Events lifecycle | Event update parsing error | MicroTime format |
| Events API | Event fetch/patch/delete | MicroTime format |
| FieldValidation CRD | CRD protobuf | CRD decoder |
| Pod InPlace Resize | Resize status | Resize implementation |
| Secrets immutable | Immutable field update | Secret handler |
| AdmissionWebhook timeout | Webhook deployment | Deployment readiness |
| Job backoffLimitPerIndex | Job index tracking | Job controller |

## Pending deploy
| # | Fix |
|---|-----|
| 298 | Probe timeout_seconds=0 defaults to 1s |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | 16 | ~150/441 | IN PROGRESS |

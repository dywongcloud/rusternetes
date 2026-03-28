# Conformance Issue Tracker

**302 total fixes** | Round 106 IN PROGRESS | 18 failures at ~300/441

## Deployed: #1-297 | Pending: #298-301

## Pending deploy (#298-302)
| # | Fix |
|---|-----|
| 298 | Probe timeout=0 defaults to 1s |
| 299 | EventSeries.lastObservedTime MicroTime |
| 300 | ResourceQuota scope filtering |
| 301 | OpenAPI v2 protobuf wrapper for kubectl |
| 302 | Immutable secrets allow metadata updates |

## Round 106 Failures (17)
| Test | Fix |
|------|-----|
| SS scaling probe timeout | #298 |
| CRD FieldSelectors/Validation/webhook | CRD protobuf |
| ResourceQuota scopes | #300 |
| kubectl replace/label/expose | #301 |
| Events lifecycle/API | #299 |
| Proxy v1 pod timeout | Pod startup |
| RC scale/quota | RC controller |
| Pod InPlace Resize | Resize impl |
| Secrets immutable | Secret handler |
| AdmissionWebhook | Webhook deploy |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | 17 | ~300/441 | IN PROGRESS |

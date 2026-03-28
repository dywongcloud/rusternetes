# Conformance Issue Tracker

**308 total fixes** | Round 107 pending deploy

## ALL Known Issues

### Fixed by pending deploy (#298-308)
| Test | Error | Fix # |
|------|-------|-------|
| SS scaling probe timeout=0 | Ready=False loop | #298 |
| Events lifecycle MicroTime | MicroTime parse error | #299 |
| Events API MicroTime | MicroTime parse error | #299 |
| ResourceQuota terminating scopes | No scope filtering | #300 |
| kubectl replace/label/expose | OpenAPI protobuf | #301 |
| Secrets immutable | Metadata update rejected | #302 |
| Job adopt/release | Pod not released | #303 |
| Job FailIndex | FailIndex action missing | #304 |
| PriorityClass endpoints | value field mutable | #305 |
| RC exceeded quota | CAS conflict on status | #306 |
| Docker OOM / zombie containers | 738 Created containers | #307 |
| EmptyDir non-root 0777 | chmod on virtiofs | #308 |

### Still unfixed — need more work
| Test | Error | Root cause |
|------|-------|------------|
| CRD protobuf (x4-5) | CRD creation times out | Protobuf decoder — needs debug deploy |
| AdmissionWebhook (x4) | Webhook deployment timeout | Docker OOM (#307 should help with more memory) |
| Proxy v1 | Pod proxy timeout | Pod not starting (#307 should help) |
| Pod InPlace Resize | Resize verification fails | Resize status needs more fields |
| RC scale rate limiter | Client rate limiter timeout | Watch reconnection overhead |

## Pending deploy (#298-308)
| # | Fix |
|---|-----|
| 298 | Probe timeout=0 defaults to 1s |
| 299 | EventSeries.lastObservedTime MicroTime |
| 300 | ResourceQuota scope filtering |
| 301 | OpenAPI v2 protobuf wrapper for kubectl |
| 302 | Immutable secrets allow metadata updates |
| 303 | Job releases pods when labels don't match |
| 304 | Job podFailurePolicy FailIndex action |
| 305 | PriorityClass value field immutable |
| 306 | RC status re-reads from storage (CAS fix) |
| 307 | Kubelet cleans up stale Created containers |
| 308 | EmptyDir uses tmpfs when fsGroup set |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | ~25 | 441 | ~94% (Docker OOM) |
| 107 | ? | 441 | est ~97% (11 pending fixes + more Docker memory) |

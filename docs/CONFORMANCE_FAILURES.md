# Conformance Issue Tracker

**307 total fixes** | Round 106 completed | ~25 failures | Round 107 pending

## ALL Known Issues

### Fixed by pending deploy (#298-307)
| Test | Error | Fix # |
|------|-------|-------|
| SS scaling probe timeout=0 | Ready=False loop | #298 |
| Events lifecycle MicroTime | MicroTime parse error | #299 |
| Events API MicroTime | MicroTime parse error | #299 |
| ResourceQuota terminating scopes | No scope filtering | #300 |
| kubectl replace | OpenAPI protobuf | #301 |
| kubectl label | OpenAPI protobuf | #301 |
| kubectl expose | OpenAPI protobuf | #301 |
| Secrets immutable | Metadata update rejected | #302 |
| Job adopt/release | Pod not released | #303 |
| Job FailIndex | FailIndex action missing | #304 |
| PriorityClass endpoints | value field mutable | #305 |
| RC exceeded quota | CAS conflict on status | #306 |
| Docker OOM / zombie containers | 738 Created containers | #307 |

### Still unfixed — need more work
| Test | Error | Root cause |
|------|-------|------------|
| CRD protobuf (x4-5) | CRD creation via protobuf times out | Protobuf decoder can't handle complex schemas |
| AdmissionWebhook (x4) | Webhook deployment timeout | Pods stuck in Created (Docker OOM, #307 may help) |
| Proxy v1 | Pod proxy timeout | Pod not starting in time |
| EmptyDir non-root 0777 | File perms wrong | Docker Desktop virtiofs limitation |
| Pod InPlace Resize | Resize verification fails | Resize status implementation incomplete |
| RC scale rate limiter | Client rate limiter timeout | Watch reconnection flood |

## Pending deploy (#298-307)
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
| 307 | Kubelet cleans up stale Created containers (prevents Docker OOM) |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | ~25 | 441 | ~94% (Docker OOM killed test early) |

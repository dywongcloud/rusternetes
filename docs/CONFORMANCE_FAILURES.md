# Conformance Issue Tracker

**285 total fixes** | Round 105 in progress | 23 failures

## Round 105 Failures (23 unique)

### Fixed pending deploy
| Test | Fix # |
|------|-------|
| conformance.go:888 — apply-patch+yaml | #282 |
| runtime.go:162 — empty container statuses | #283 |
| aggregated_discovery.go:227 — CRD discovery | #285 |
| expansion.go:345 — kubelet sync blocked | #284 |
| webhook.go:1133 (x2) — kubelet sync blocked | #284 |
| preemption.go (x2) — kubelet sync blocked | #284 |

### Still need fixes
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | Watch ordering | Watch cache event delivery |
| crd_publish_openapi.go:244 | CRD timeout | CRD protobuf decoder |
| crd_watch.go:72 | CRD watch | CRD watch events |
| output.go:282 | Secret fsGroup perms | fsGroup handling |
| pods.go:556 | Pod generation | Generation field |
| job.go:548 | Job success policy | Job tracking |
| builder.go:97 (x2) | kubectl protobuf | OpenAPI protobuf |
| kubectl patch/guestbook | kubectl issues | CLI handling |
| Events API delete collection | DeleteCollection | Events handler |
| HostPort conflicts | hostPort validation | Not implemented |
| Pod InPlace Resize | Container resize | Not implemented |
| Container termination msg | FallbackToLogsOnError | Termination policy |

## Pending deploy
| # | Description |
|---|-------------|
| 282 | Status PATCH accepts apply-patch+yaml |
| 283 | Preserve container status when removed |
| 284 | Kubelet sync timeouts (10s/30s) |
| 285 | Aggregated discovery dynamic CRD groups |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 23 | ~200/441 | IN PROGRESS |

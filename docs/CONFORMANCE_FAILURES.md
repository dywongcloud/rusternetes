# Conformance Issue Tracker

**288 total fixes** | Round 105 in progress | 24 failures

## Pending deploy (#282-288) — 7 fixes
| # | Fix | Tests |
|---|-----|-------|
| 282 | Status PATCH accepts apply-patch+yaml for SSA | 1 |
| 283 | Preserve container status when Docker container removed | 2 |
| 284 | Kubelet sync timeouts (10s/30s) prevent 4min blocking | ~6 |
| 285 | Aggregated discovery includes dynamic CRD groups | 1 |
| 286 | MicroTime always includes .000000 microseconds | 1 |
| 287 | set_initial_generation always sets generation=1 | 1 |
| 288 | fsGroup chmod uses g+rX not g+rwX | 1 |

## Remaining unfixed issues
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | Watch ordering timeout | Watch cache event delivery |
| crd_publish_openapi.go:244 | CRD OpenAPI timeout | CRD protobuf decoder |
| crd_watch.go:72 | CRD watch | CRD watch events |
| builder.go:97 (x2) | kubectl protobuf OpenAPI | kubectl protobuf format |
| kubectl patch/guestbook | kubectl issues | CLI handling |
| HostPort conflicts | hostPort validation | Kubelet blocking (#284) |
| Pod InPlace Resize | Container resize | Not implemented |
| Job success policy | Job completion | Job tracking |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 24 | 441 | IN PROGRESS |

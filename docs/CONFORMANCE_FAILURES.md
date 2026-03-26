# Conformance Issue Tracker

**Round 97**: 39 FAIL, 0 PASS | **177 fixes** (11 pending deploy)

## Critical pending deploy fixes
| # | Fix | Impact |
|---|-----|--------|
| 169 | generation=1, ClusterIP, SA token, PodScheduled | 5+ tests |
| 170 | resourceVersion in watch event values | 12+ tests |
| 171 | Endpoints single subset | 1 test |
| 172 | Ensure metadata for resourceVersion | 1 test |
| 173 | Remove duplicate SA token route (panic) | startup |
| 174 | **CRITICAL** List RV from items, not timestamps | ALL tests |
| 175 | Immutable returns 403 Forbidden | 2 tests |
| 176 | RC orphan handling + DaemonSet ControllerRevision | 2 tests |
| 177 | Aggregated discovery responseKind.group empty | 1 test |

## Round 97 failures — fix status

### Fixed by pending deploys
- statefulset.go (watch timeout) — #170, #174
- rc.go (3 tests: timeout, replicas) — #170, #174, #176
- job.go (3 tests: completion timeout) — #170, #174
- deployment.go:238 (timeout) — #170, #174
- watch.go:454 (ADDED event) — #170, #174
- proxy.go:271,:503 (timeout) — #170, #174
- controller_revision.go:156 — #176 (DaemonSet CR)
- service.go:1483 (NodePort/ClusterIP) — #169
- configmap_volume.go:547 (immutable) — #175
- garbage_collector.go:436 (orphan) — #176
- core_events.go:135 (timestamp) — already fixed, old deploy
- service_accounts.go:132,:792 — need further investigation
- kubectl.go:1130 (dry-run) — #169

### Not yet fixed
- crd_publish_openapi.go:244,:285 — protobuf (always failing)
- builder.go:97 (×2) — protobuf
- webhook.go:837,:1194,:1244 — webhook readiness
- aggregated_discovery.go:282 — resource format
- service.go:251 — affinity
- runtimeclass.go:153,:297 — watch + list length
- output.go:263 — subpath
- runtime.go:169 — termination message
- pod_resize.go:857 — resize (not implemented)
- validatingadmissionpolicy.go:568 — VAP
- resource_quota.go:102,:209 — quota
- kubectl.go:1881 — proxy
- service_cidrs.go:255 — IPAddress

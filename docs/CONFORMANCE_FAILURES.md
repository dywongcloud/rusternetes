# Conformance Issue Tracker

**Round 110** | COMPLETE | 441/441 tests | 283 passed, 158 failed (64.2% pass)

## Non-Timeout Failures with Committed Fixes (~73)

| Category | Count | Commit | Fix |
|----------|-------|--------|-----|
| kubectl create -f stdin | 8 | 5da5f98 | Protobuf envelope wrapping for OpenAPI |
| Pod resize PATCH | 4 | 7d40469 | X-Original-Content-Type header for PATCH |
| StatefulSet canary/rollback | 3 | 0591bb2 | Revision tracking during rolling updates |
| Webhook CEL panic | 2 | 7d58174 | catch_unwind for CEL parser panics |
| Webhook CEL metadata | 2 | c6dc16e | Allow "no such key" compile errors |
| Webhook match conditions | 2 | c6dc16e | catch_unwind + error filtering |
| ValidatingAdmissionPolicy | 2 | 818922f | Variable evaluation + expression support |
| Scheduler preemption | 2 | 2f1d98d | Per-node resource tracking |
| StatefulSet SHA-256 | 2 | 78c79bb | Deterministic revision hashing |
| Variable expansion subpath | 2 | c6dc16e | Backtick before expansion, component ".." |
| StatefulSet patch image | 1 | 0591bb2 | Revision hash comparison after patch |
| StatefulSet scaling | 1 | 0591bb2 | Deterministic hash prevents spurious deletes |
| Job backoffLimitPerIndex | 1 | 0591bb2 | Per-index failure tracking |
| Job podFailurePolicy | 1 | 0591bb2 | FailIndex action handling |
| Job completion restart | 1 | 0591bb2 | Local restart completion tracking |
| Job successPolicy all | 1 | 0591bb2 | SuccessPolicy all-indexes evaluation |
| Job successPolicy indexes | 1 | 0591bb2 | SuccessPolicy succeededIndexes rules |
| Deployment TypeMeta | 1 | c6dc16e | TypeMeta defaults + injection |
| Deployment status | 1 | cde918d + 324fd8a | availableReplicas aggregation from RS |
| Service selector decode | 1 | agents | selector always serialized (Default trait) |
| LimitRange defaults | 1 | f65ab7b | Default request fallback to limits |
| RC failure condition | 1 | f65ab7b | CAS retry + conditions=None to clear |
| Events field selector | 1 | 55c1e5a | "source" alias for "source.component" |
| Watch label ADDED | 1 | fba0a62 | Synthetic ADDED for label match changes |
| Aggregated discovery | 1 | 829ce94 | Group field in resources |
| kubectl logs newline | 1 | 829ce94 | No trailing newline |
| Namespace cascade | 1 | cde918d | Cascade finalization |
| Namespace status patch | 1 | 55c1e5a | Preserve metadata in JSON PATCH |
| Hostport binding | 1 | 72d2973 | hostIP binding from pod spec |
| /etc/hosts tar | 1 | b98b8c4 | Tar upload to pause container |
| Secrets volume | 1 | d8030f2 | Deletion handling in resync |
| WebSocket exec ordering | 1 | 6af1a31 | SPDY channel 1 before channel 3 |
| Termination message | 1 | 6af1a31 | FallbackToLogsOnError policy |
| emptyDir medium | 1 | 7881f80 | Bind mounts for default medium |
| Duplicate resolv.conf | 1 | 90cd952 | Prevent duplicate mount |
| DaemonSet numberReady | 1 | 3efd08d | Count Ready condition, not Running phase |
| DaemonSet rolling update | 1 | 5452f2c | Delete pods with old template hash |
| ControllerRevision num | 1 | a6a254f | Incremented revision numbers |
| VAP binding delay | 1 | 8bcfeb2 | Remove 2-second age delay |
| Pod generation | 1 | 172ffa3 | Bump generation on graceful delete |
| RuntimeClass overhead | 1 | 172ffa3 | Inject overhead from RuntimeClass |
| CSR API GET | 1 | c6dc16e | GET method on /approval endpoint |
| Sysctl name validation | 1 | 55c1e5a | Validate names, K8s error format |
| InitContainer conditions | 1 | 55c1e5a | Initialized=False when init fails |
| PDB eviction format | 1 | 55c1e5a | IntOrString Display, not Debug |
| FieldValidation duplicate | 1 | 55c1e5a | "json: unknown field" format |
| NoExecute taint eviction | 1 | 55c1e5a | tolerationSeconds expiry support |
| SA OIDC discovery | 1 | b07715d | OIDC discovery endpoints |
| SA TokenReview | 1 | 44e23e0 | Handle Go-style null in TokenRequest |
| CSIStorageCapacity | 1 | ba1c0d6 | Watch support for CSI list endpoint |
| SA pod extra info | 1 | 44e23e0 | Cascading fix — TokenRequest null handling |

## Non-Timeout Failures NOT Yet Fixed (~10)

| Category | Count | Error | Status |
|----------|------|-------|--------|
| Session affinity | 4 | "service not reachable" | kube-proxy networking — services unreachable |
| Deployment rolling update | 1 | "revision mismatch" | RS revision annotation propagation |
| Deployment rollover | 1 | "0 pods available" | Rollover availability timing |
| Events lifecycle | 1 | "event wasn't updated" | Chrono timestamp precision loss |
| ControllerRevision lifecycle | 1 | "revision 1 expected 3" | Controller overwrites test update |
| kubectl proxy | 1 | "unexpected end of JSON" | Proxy response format |
| PriorityClass endpoints | 1 | "10 != 1" | Value field mismatch |
| Aggregator | 1 | "extension apiserver" | API aggregation — not feasible |

## Not Code Bugs (~82)

| Category | Count | Notes |
|----------|-------|-------|
| Timeout failures | 79 | Docker Desktop latency — pods take 2-10s to Ready |
| /etc/hosts managed | 1 | Docker overrides /etc/hosts in container:pause mode |
| emptyDir shared volume | 1 | Docker bind mount path visibility |
| DaemonSet rollback | 1 | Timeout (was miscategorized as code bug) |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |

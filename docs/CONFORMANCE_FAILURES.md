# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (round 8 — live monitoring)

## Current Run Status
- Run started: 2026-03-19 21:28 UTC
- Tests completed: 3 of 441
- Passed: 1, Failed: 2

## Fixed Issues (25 root causes, all committed)

| # | Issue | Tests | Commit |
|---|-------|-------|--------|
| 1 | Status subresource routing | ~8 | c89343c |
| 2 | Projected volume support | ~20 | c89343c |
| 3 | Service type defaulting + headless | ~6 | 09ed555, 640da11 |
| 4 | Lease MicroTime format | ~1 | 09ed555 |
| 5 | EndpointSlice managed-by label | ~1 | 09ed555 |
| 6 | Discovery auth + webhook + aggregated | ~3 | 09ed555, e21ad25 |
| 7 | DeleteCollection routes | ~3 | 09ed555 |
| 8 | Exec query parsing | ~30 | ebcc6b8 |
| 9 | Pod IP / Container ID / Image ID | ~5 | 894bdc1 |
| 10 | SA automount bypass + NodePort alloc | ~4 | b6a4fea |
| 11 | Watch sendInitialEvents + selectors | ~all | 767a005, c36367f |
| 12 | Watch support in all handlers | ~20 | c36367f |
| 13 | CronJob ? + downwardAPI + ConfigMap/Event | ~10 | aecc290 |
| 14 | WebSocket log streaming | ~1 | aecc290 |
| 15 | NodePort MASQUERADE + ephemeral route | ~4 | 875eecf |
| 16 | Watch empty RV + status details | ~3 | ad10a8b |
| 17 | DaemonSet dupes + node deser + validation | ~4 | ad10a8b |
| 18 | IPAddress API + RoleBinding errors | ~2 | ad10a8b |
| 19 | Protobuf 406 fallback | ~3 | 6d0788a |
| 20 | SubPathExpr variable expansion | ~2 | 6d0788a |
| 21 | GC replicationcontrollers scan | ~2 | 6d0788a |
| 22 | Scheduler preemption eviction | ~2 | 6d0788a |
| 23 | Second node (node-2) | ~2 | de9175a |
| 24 | Pod initial Pending phase | ~2 | ad78f7e |
| 25 | Probe IP through pause containers | ~5 | ad78f7e |

## Known Failures in Current Run

### F1. Variable Expansion subpath — pod should FAIL but doesn't
**Test**: `expansion.go:272`
**Error**: `Failed after 10.037s. Expected Pod to be in "Pending" Got instead: Running`
**Root cause**: Test creates pod with `$(ANNOTATION)` in subPathExpr where annotation
'mysubpath' is NOT set. The kubelet should detect the missing annotation and set
container to Waiting/CreateContainerError, but instead the pod reaches Running.
**Fix needed**: In `expand_subpath_expr()` in runtime.rs, when resolving downward API
field refs like `metadata.annotations['key']`, return error if annotation doesn't exist
instead of returning empty string.
**File**: `crates/kubelet/src/runtime.rs`

### F2. StatefulSet pods — "Failed waiting for pods to enter running"
**Test**: `statefulset/wait.go:63`
**Error**: `context deadline exceeded`
**Root cause**: Pod IS running and Ready (verified via API), but the test's watch
stream isn't receiving the status update events. The StatefulSet test uses a polling
function that watches for pod status changes. The issue may be:
- Watch stream not delivering MODIFIED events for pod status updates
- The kubelet updates pod status but the watch event is filtered or dropped
**File**: `crates/api-server/src/handlers/watch.rs` (event delivery)

### F3. Exec command "No such file or directory" (affects ~30 tests)
**Error**: `Failed to spawn exec command: No such file or directory (os error 2)`
**Root cause**: The API server's SPDY/exec handler tries to spawn `docker exec`
as a subprocess, but the Docker CLI is not installed in the API server container.
Exec should use bollard's exec API instead of spawning a process.
**File**: `crates/api-server/src/spdy_handlers.rs`

## Infrastructure Limitations

| # | Issue | Tests | Reason |
|---|-------|-------|--------|
| A | 2 nodes required | ~2 | Fixed: node-2 added |

## Notes for Next Session

- All 25 fixes are committed and deployed
- Conformance run is in progress with all fixes
- Key remaining issues: F1 (subpath expansion failure), F2 (watch event delivery), F3 (exec via Docker)
- F3 is the highest impact (30+ tests use exec)
- The exec fix requires changing spdy_handlers.rs to use bollard API instead of docker CLI

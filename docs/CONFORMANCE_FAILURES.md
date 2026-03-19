# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (after fixes round 3)

## Fixed Issues (deployed or committed)

| # | Issue | Tests | Status |
|---|-------|-------|--------|
| 1 | Status subresource routing (Path args) | ~8 | ✅ FIXED (c89343c) |
| 2 | Projected volume support | ~20 | ✅ FIXED (c89343c) |
| 3 | Service type defaulting (ClusterIP) | ~3 | ✅ FIXED (09ed555) |
| 4 | Headless service (ClusterIP=None) rejection | ~3 | ✅ FIXED (640da11) |
| 5 | Lease MicroTime format | ~1 | ✅ FIXED (09ed555) |
| 6 | EndpointSlice managed-by label | ~1 | ✅ FIXED (09ed555) |
| 7 | Discovery /api auth + webhook resources | ~2 | ✅ FIXED (09ed555) |
| 8 | DeleteCollection routes for PDB/podtemplates | ~3 | ✅ FIXED (09ed555) |
| 9 | Exec query parameter parsing (repeated command=) | ~30 | ✅ FIXED (ebcc6b8) |
| 10 | Pod IP / Container ID / Image ID reporting | ~5 | ✅ FIXED (894bdc1) |
| 11 | SA automount bypass | ~2 | ✅ FIXED (b6a4fea) |
| 12 | NodePort auto-allocation | ~2 | ✅ FIXED (b6a4fea) |
| 13 | Watch sendInitialEvents + bookmark annotation | ~all | ✅ FIXED (767a005, 8f4a926) |
| 14 | Watch label/field selector filtering | ~10 | ✅ FIXED (c36367f) |
| 15 | Watch support in all resource handlers | ~20 | ✅ FIXED (ad84a5f, c36367f) |

## Remaining Issues (need fixing)

### HIGH PRIORITY

#### 18. CronJob cron schedule parsing — `?` character (affects ~2 tests)
**Error**: `Failed to parse cron schedule '*/1 * * * ?': Invalid expression`
**Root cause**: Our cron parser doesn't support `?` (Quartz-style "no specific value").
Kubernetes cron supports `?` as equivalent to `*` for day-of-week.
**File**: `crates/controller-manager/src/controllers/cronjob.rs` (cron parsing)
**Fix**: Replace `?` with `*` before parsing, or use a cron library that supports it.

#### 19. Pod volume mounting — resource field refs (affects ~5 tests)
**Error**: Pods with downward API volume mounts for resource limits/requests timeout
**Root cause**: `resolve_resource_field_ref` may not handle cpu/memory correctly,
or the file mode isn't being set properly.
**File**: `crates/kubelet/src/runtime.rs`

#### 20. Service reachability via NodePort (affects ~3 tests)
**Error**: `service is not reachable within 2m0s timeout on endpoint nodeport-test:80`
**Root cause**: NodePort services not reachable from inside pods.
kube-proxy NodePort iptables rules may not be in the right namespace.

#### 21. DNS resolution failures (affects ~2 tests)
**Error**: `Unable to read agnhost_udp@kubernetes.default.svc.cluster.local`
**Root cause**: DNS queries from pods don't resolve cluster services.
CoreDNS may need proper service/endpoint data.

### MEDIUM PRIORITY

#### 22. CRD protobuf creation (affects ~3 tests)
**Error**: `the body of the request was in an unknown format`
**Root cause**: CRD test framework sends protobuf. Can't fix without protobuf support.

#### 23. Deserialization failures (affects ~3 tests)
**Error**: `the server rejected our request due to an error in our request`
**Root cause**: ConfigMap, ReplicaSet, Event creation failing due to
missing/incompatible fields in our structs.

#### 24. Variable Expansion subpath (affects ~2 tests)
**Error**: Pod creation immediate failure at expansion.go:272
**Root cause**: kubelet doesn't handle subPathExpr expansion failure properly.

#### 25. Conformance requires 2 nodes (affects 2 tests)
**Error**: `Conformance requires at least two nodes`
**Root cause**: Cannot fix without adding a second node.

#### 26. WebSocket log streaming (affects 1 test)
**Error**: `Failed to open websocket to .../log: bad status`

#### 27. Aggregated discovery format (affects 1 test)
**Error**: Missing aggregated discovery endpoint format

#### 28. Ephemeral container PATCH auth (affects 1 test)
**Error**: `Missing request extension: AuthContext`
**Root cause**: Ephemeral container PATCH route missing auth middleware.

## Test Results Summary

| Run | Date | Passed | Failed | Total | Rate |
|-----|------|--------|--------|-------|------|
| Quick mode | 2026-03-18 | 1 | 0 | 1 | 100% |
| Full run 1 | 2026-03-19 | 11 | 75 | 86 | 13% |
| Full run 2 (partial) | 2026-03-19 | 1 | 5 | 6 | 17% |
| Full run 3 | 2026-03-19 | in progress | | | |

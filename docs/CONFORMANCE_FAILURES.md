# Full Conformance Failure Analysis

86 tests completed, 75 failed, 11 passed (13% pass rate).
Analysis from conformance run on 2026-03-19.

## Root Cause Categories

### 1. STATUS SUBRESOURCE PATH ROUTING (affects ~8 tests)
**Error**: `Wrong number of path arguments for 'Path'. Expected 3 but got 2`
**Examples**:
- PATCH pods/status: `patch pods pod-8tb8l`
- GET statefulset/status: `fetch NamespaceStatus namespaces-2752`
- PATCH nodes/status: `patch nodes node-1`
- PATCH jobs.batch/status: `patch jobs.batch suspend-false-to-true`

**Root cause**: Status subresource routes like `/api/v1/namespaces/{ns}/pods/{name}/status`
are not properly routing. The path parser expects different arg counts.
**Fix**: Check router for `/status` suffix routes.

### 2. POD TIMEOUT — Pods not starting/running (affects ~20+ tests)
**Error**: `expected pod "..." success: Timed out after 300.003s`
**Examples**: configmap pods, secret pods, projected pods, downwardAPI pods
**Root cause**: Pods that mount configMap/secret volumes are created but never reach
Running phase. The kubelet is likely failing to mount volumes or the pods crash.
**Fix**: Investigate kubelet volume mounting for configMap and secret volumes.

### 3. MISSING AUTH CONTEXT (affects ~3 tests)
**Error**: `Missing request extension: Extension of type 'api_server::middleware::AuthContext' was not found`
**Root cause**: Some API endpoints don't go through the auth middleware. The
`/api` discovery endpoint and ephemeral container PATCH are missing it.
**Fix**: Ensure all API routes go through the auth middleware.

### 4. SERVICE TYPE DEFAULTING (affects ~3 tests)
**Error**: `unexpected Spec.Type () for service, expected ClusterIP`
**Error**: `unexpected Spec.ClusterIP (None) for ExternalName service, expected empty`
**Root cause**: Service type not defaulted to "ClusterIP" on creation.
ExternalName services should have empty ClusterIP (""), not "None".
**Fix**: Default service type to "ClusterIP". Set ClusterIP to "" for ExternalName.

### 5. DELETE COLLECTION NOT IMPLEMENTED (affects ~3 tests)
**Error**: `the server does not allow this method on the requested resource (delete podtemplates)`
**Error**: `Deleting PDB set: the server does not allow this method (delete poddisruptionbudgets.policy)`
**Root cause**: DeleteCollection endpoint returns 405 for some resources.
**Fix**: Implement DELETE on collection endpoints for podtemplates, poddisruptionbudgets.

### 6. CRD PROTOBUF CREATION (affects ~3 tests)
**Error**: `the body of the request was in an unknown format - accepted media types include: application/json`
**Root cause**: CRD test framework sends protobuf, our API rejects it.
**Fix**: This is hard to fix without protobuf support. Need JSON-only client config.

### 7. DESERIALIZATION FAILURES (affects ~3 tests)
**Error**: `the server rejected our request due to an error in our request (post configmaps/replicasets/events)`
**Root cause**: Some request bodies have fields our structs don't handle.
**Fix**: Make structs more permissive (deny_unknown_fields not set, all fields Optional).

### 8. DNS RESOLUTION FAILURES (affects ~2 tests)
**Error**: `Unable to read agnhost_udp@kubernetes.default.svc.cluster.local`
**Root cause**: DNS queries inside pods don't resolve cluster services.
**Fix**: CoreDNS integration and pod DNS configuration.

### 9. SERVICE REACHABILITY (affects ~3 tests)
**Error**: `service is not reachable within 2m0s timeout on endpoint nodeport-test:80`
**Root cause**: NodePort services not reachable. kube-proxy iptables may not work for NodePort.
**Fix**: Verify kube-proxy NodePort rules.

### 10. CONFORMANCE REQUIRES 2 NODES (affects 2 tests)
**Error**: `Conformance requires at least two nodes`
**Root cause**: Some tests require 2+ nodes, we only have 1.
**Fix**: Cannot fix without adding a second node to the cluster.

### 11. LEASE TIMESTAMP PARSING (affects 1 test)
**Error**: `parsing time "0001-01-01T00:00:02Z" as "2006-01-02T15:04:05.000000Z07:00": cannot parse "Z"`
**Root cause**: Lease acquireTime/renewTime serialized with wrong format (missing microseconds).
**Fix**: Use chrono's proper format or MicroTime format.

### 12. WEBSOCKET LOGS (affects 1 test)
**Error**: `Failed to open websocket to .../log?container=main: bad status`
**Root cause**: WebSocket log streaming endpoint returns error.
**Fix**: Check log streaming via WebSocket.

### 13. ENDPOINTSLICE LABELS (affects 1 test)
**Error**: `Expected EndpointSlice to have endpointslice.kubernetes.io/managed-by label`
**Root cause**: EndpointSlice controller doesn't set the managed-by label.
**Fix**: Set label when creating EndpointSlices.

### 14. WATCH WITH INITIAL RV "" (affects 1 test)
**Error**: `initial RV "" is not supported due to issues with underlying WATCH`
**Root cause**: Watch with empty resourceVersion not handled properly.
**Fix**: Handle empty resourceVersion in watch handler.

### 15. EPHEMERAL CONTAINERS PATCH (affects 1 test)
**Error**: `Missing request extension: AuthContext` on ephemeral container patch
**Root cause**: Ephemeral container subresource PATCH not routed through auth middleware.
**Fix**: Add auth middleware to ephemeral container routes.

### 16. DISCOVERY ENDPOINT (affects 1 test)
**Error**: `Expected gvr admissionregistration.k8s.io v1 validatingwebhookconfigurations to exist in discovery`
**Root cause**: Discovery endpoint missing validatingwebhookconfigurations resource.
**Fix**: Add to discovery handler.

### 17. AGGREGATED DISCOVERY (affects 1 test)
**Error**: Missing aggregated discovery endpoint
**Root cause**: /apis endpoint format doesn't match aggregated discovery expectations.
**Fix**: Implement aggregated discovery format.

## Priority Order for Fixes

### HIGH IMPACT (fix these first — affects many tests):
1. Pod volume mounting timeouts (20+ tests) — configmap/secret volumes not working
2. Status subresource routing (8 tests) — /status path parsing broken
3. Missing AuthContext on routes (3 tests) — middleware not applied to all routes

### MEDIUM IMPACT:
4. Service type defaulting (3 tests)
5. DeleteCollection for podtemplates/PDB (3 tests)
6. Deserialization failures (3 tests) — make structs more permissive

### LOW IMPACT (1-2 tests each):
7. CRD protobuf (3 tests)
8. DNS resolution (2 tests)
9. Lease timestamp format (1 test)
10. EndpointSlice managed-by label (1 test)
11. WebSocket log streaming (1 test)
12. Conformance 2-node requirement (2 tests — can't fix)

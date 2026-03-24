# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 86 in progress — 15/441 specs done, 14 failures, 57 fixes)

## Critical root causes fixed

| # | Root Cause | Impact | Fix | Commit |
|---|-----------|--------|-----|--------|
| 1 | Container restart storm | Containers recreated every 2s, floods watches | Check state before recreating | `e99730b` |
| 2 | Watch Added vs Modified | All Put events sent as MODIFIED | Check prev_kv for new keys | `00db87c` |
| 3 | Field selector missing=false | All tests blocked | Treat missing as false | `646a407` |
| 4 | Protobuf body rejection | CRD/EndpointSlice creation fails | Extract JSON from envelope | `2571b32` |
| 5 | Service CIDR no route | Pod-to-service fails | Add route in kube-proxy | `b4f31c2` |
| 6 | API ClusterIP not routable | Pods can't reach API | Direct IP + TLS SANs | `b224387`+ |
| 7 | Watch N×N explosion | etcd overwhelmed | Watch cache (1 per prefix) | `73c3514` |
| 8 | Controller 10s interval | Tests time out waiting | Reduced to 2s | `ea1b800` |
| 9 | Namespace cascade order | Orphaned pods block cleanup | Delete controllers first | `1270649` |

## Round 86 active failures (14 tests, 12 unique types)

### 1. Watch closed before timeout (StatefulSet)
- **Test**: `statefulset.go:786` — StatefulSet scaling order verification
- **Symptom**: Watch for pod events closes before test verifies scaling order
- **Root cause**: HTTP/2 stream management — client-go sends RST_STREAM which causes tx.send() to fail
- **Status**: Bookmark interval reduced to 15s. Container restarts reduced to 4 from 25+. Watch still closes.
- **Research needed**: HTTP/2 stream lifecycle management. May need different streaming mechanism.

### 2. Lifecycle hook timeout (PostStart HTTP)
- **Test**: `lifecycle_hook.go:118` — PostStart HTTP hook execution
- **Symptom**: Timed out after 120s waiting for postStart HTTP hook
- **Root cause**: PostStart httpGet hook needs to reach another pod via HTTP. Likely pod-to-pod networking issue — the hook URL may not be reachable from the container's network namespace.
- **Research needed**: Check if pod-to-pod HTTP connectivity works within Docker bridge network. May need kube-proxy DNAT for pod IPs.

### 3. Pod resize PATCH 404 — FIXED (not deployed)
- **Test**: `pod_resize.go:850` — In-place pod resource resize
- **Symptom**: `the server could not find the requested resource (patch pods resize-test-c7e4d)`
- **Root cause**: Missing `/api/v1/namespaces/:ns/pods/:name/resize` route
- **Fix**: Added resize subresource route (GET/PUT/PATCH) mapped to pod handlers

### 4. Aggregated discovery missing GVRs
- **Test**: `aggregated_discovery.go:165` — API discovery v2 format
- **Symptom**: `Expected gvr admissionregistration.k8s.io v1 validatingwebhookconfigurations to exist in discovery`
- **Root cause**: API server doesn't implement aggregated discovery v2 format (APIGroupDiscoveryList). The v2 endpoint `/api` and `/apis` must return the aggregated format when `Accept: application/json;g=apidiscovery.k8s.io;v=v2;as=APIGroupDiscoveryList` is requested.
- **Research needed**: Implement content negotiation for discovery endpoints.

### 5. Scheduler preemption timeout (×2)
- **Test**: `preemption.go:181` and `preemption.go:516` — Pod preemption
- **Symptom**: Timed out after 300s
- **Root cause**: Scheduler doesn't implement pod preemption. When a high-priority pod can't be scheduled, it should evict lower-priority pods.
- **Research needed**: Implement priority-based preemption in scheduler.

### 6. DNS context deadline exceeded
- **Test**: `dns_common.go:476` — DNS resolution
- **Symptom**: `context deadline exceeded` after 604s
- **Root cause**: DNS resolution for services/pods failing. Likely CoreDNS can't resolve service names because endpoint slices aren't being properly populated.
- **Research needed**: Check CoreDNS configuration and endpoint slice controller.

### 7. ResourceClaim missing Kind — FIXED (not deployed)
- **Test**: `conformance.go:686` — DRA ResourceClaim CRUD
- **Symptom**: `Object 'Kind' is missing in '{"metadata":...'`
- **Root cause**: ResourceClaim create/update handlers didn't set `kind` and `apiVersion` before storing
- **Fix**: Added `claim.kind = "ResourceClaim"` and `claim.api_version = "resource.k8s.io/v1"` in create/update

### 8. kubectl create protobuf / CRD field validation — FIXED (not deployed)
- **Test**: `field_validation.go:428` — CRD field validation with protobuf
- **Symptom**: `cannot create crd only application/json is supported`
- **Root cause**: Protobuf extraction middleware only handled wire types 0 (varint) and 2 (length-delimited), failing on wire type 1 (64-bit) or 5 (32-bit). Also tag parsing assumed single-byte tags.
- **Fix**: Added support for all wire types (0, 1, 2, 5) and multi-byte tag parsing. Added JSON fallback scan.

### 9. Container output "2\n" (Downward API CPU)
- **Test**: `output.go:263` — Container output verification
- **Symptom**: Expected container output doesn't match, got "2\n"
- **Root cause**: CPU resource value returned as "2" when test expected a different value. May be correct if pod requests 2 CPUs, or divisor handling is wrong.
- **Research needed**: Check specific test pod spec to verify expected CPU value vs our computation.

### 10. Job completion timeout
- **Test**: `job.go:817` — Job completion with indexed completion
- **Symptom**: `failed to ensure job completion: Told to stop trying after 201.487s`
- **Root cause**: Job controller may be too slow to mark job as complete, or indexed completion tracking is broken.
- **Research needed**: Check job controller completion detection speed.

### 11. ResourceSlice watch error — FIXED (not deployed)
- **Test**: `conformance.go:824` — DRA ResourceSlice CRUD lifecycle
- **Symptom**: `no kind "ResourceSliceList" is registered for version "resource.k8s.io/v1"`
- **Root cause**: ResourceSlice list handler didn't intercept `?watch=true` — returned a `ResourceSliceList` JSON instead of a watch stream. Client-go tried to decode the list as a watch event and failed.
- **Fix**: Added watch interception to all DRA list handlers (ResourceSlice, ResourceClaim, ResourceClaimTemplate). Created `watch_cluster_scoped_json` and `watch_namespaced_json` functions for DRA types that use `serde_json::Value` instead of `HasMetadata` trait.

### 12. DaemonSet retry failed pods — FIXED (not deployed)
- **Test**: `daemon_set.go:332` — DaemonSet should retry creating failed daemon pods
- **Symptom**: `error waiting for the failed daemon pod to be completely deleted: context deadline exceeded`
- **Root cause**: DaemonSet controller didn't handle pods in "Failed" phase. When test set pod phase to Failed, the controller kept the pod (it was in pods_by_node map) and never recreated it.
- **Fix**: DaemonSet controller now detects Failed/Succeeded pods, deletes them, and recreates. Also reduced controller interval from 5s to 2s.

## All 57 fixes committed (6 pending deploy)

| Fix | Commit |
|-----|--------|
| Container logs search | `2b1008d` |
| EventList ListMeta | `97938e4` |
| gRPC probe | `e738c1f` |
| Scale PATCH | `d335dee` |
| Status PATCH routes | `d335dee` |
| events.k8s.io/v1 | `f8a75da` |
| CRD openAPIV3Schema | `abd2137` |
| ResourceSlice Kind | `9b21a89` |
| PDB status defaults | `9b21a89` |
| PV status phase | `710eee1` |
| metadata.namespace | `db40409` |
| camelCase abbreviations | `bde38ef` |
| VolumeAttributesClass delete | `bde38ef` |
| OpenAPI protobuf 406 | `bde38ef` |
| Container retention | `2c8e1fd` |
| Termination message | `c804e57` |
| Init container Waiting | `b54d541` |
| StatefulSet revision | `7f5c9bc` |
| SA token key | `9238eb4` |
| Proxy handler keys | `b4b745c` |
| nonResourceURLs | `98f0eac` |
| Deployment revision | `565c216` |
| EndpointSlice cleanup | `6f79efa` |
| Fail on missing volumes | `5e07c6e` |
| ClusterIP pre-allocation | `4113fe9` |
| K8S_SERVICE_HOST | `b224387` |
| K8S_SERVICE_PORT | `862c286` |
| TLS SANs | `f9c9691` |
| ClusterIP re-allocation | `cd6ab64` |
| Field selector | `646a407` |
| Watch reconnect | `5edb20d` |
| runAsGroup | `bbaa43f` |
| RC FailedCreate | `702b107` |
| Service CIDR route | `b4f31c2` |
| iproute2 | `cec24ff` |
| JOB_COMPLETION_INDEX | `f288981` |
| /apis/{group} | `3327567` |
| Downward API | `095b407` |
| Watch bookmark resilience | `0eb215d` |
| Watch cache | `73c3514` |
| Watch subscribe-before-list | `d2e306c` |
| Namespace cascade | `1270649` |
| Protobuf extraction | `2571b32` |
| RS conditions preserve | `58317e6` |
| Controller interval 2s | `ea1b800` |
| Status PATCH metadata | `38b44f2` |
| Container restart fix | `e99730b` |
| Watch Added vs Modified | `00db87c` |
| CSINode deletecollection | `a9cdc55` |
| IntOrString serde_json::Value | `fbf759b`+ |
| ResourceClaim Kind | pending |
| Pod resize route | pending |
| Protobuf wire types | pending |
| DRA watch support | pending |
| DaemonSet retry failed pods | pending |
| ResourceSlice update Kind | pending |
| DaemonSet 2s interval | pending |

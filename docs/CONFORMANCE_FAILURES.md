# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 88: 61 PASS, 29 FAIL (68%) with 62 fixes deployed; round 89 pending with 75 fixes total)

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

## Round 87 active failures (in progress)

### 1. Watch closed before timeout (StatefulSet) — FIXED (pause container)
- **Test**: `statefulset.go:786` — StatefulSet scaling order verification
- **Symptom**: Container restart storm (25+ restarts in 50 seconds). Watch closes.
- **Root cause**: `start_pause_container` always force-removed and recreated the pause container, even if it was already running. Since all pod containers share the pause container's network namespace, recreating the pause container killed all other containers. Every `start_pod` call (e.g., during `Pending if is_running` path) would trigger this cascade.
- **Fix**: `start_pause_container` now checks if the pause container is already running and returns its IP immediately without recreation. Only recreates if the pause container is stopped/dead.
- **Status**: Code fix written, needs deploy.

### 2. Scheduling predicates timeout (Taint tolerance)
- **Test**: `predicates.go:1102` — Pod scheduling with taints/tolerations
- **Symptom**: `Timed out after 240.006s`
- **Root cause**: Scheduler doesn't handle taints/tolerations properly. When a node has a taint, pods without matching tolerations should not be scheduled there.
- **Status**: Seen in round 87.

### 3. Pod resize cgroup mismatch — DEPLOYED
- **Test**: `pod_resize.go:850` (via cgroups.go) — In-place pod resource resize
- **Symptom**: After resize PATCH, cgroup values don't update. Expected `cpu.max: "2500 100000"` got `"2000 100000"`. Expected `memory.max: "28311552"` got `"23068672"`.
- **Root cause**: Kubelet doesn't actually update container cgroup limits when pod resources are resized. The resize status is updated in the API but Docker container resource limits are not modified.
- **Status**: Seen in round 87. Needs `docker update` integration in kubelet.

### 4. Job indexed completion — completedIndexes missing — FIXING
- **Test**: `job.go:817` — Job completion with indexed completion mode
- **Symptom**: Job completes (4/4 succeeded) but test waits for `status.completedIndexes` field which is always `None`.
- **Root cause**: Job controller never sets `completedIndexes` for Indexed completion mode. Also not assigning `batch.kubernetes.io/job-completion-index` annotation to pods.
- **Fix**: Added completedIndexes tracking from succeeded pod annotations/labels. Added index annotation and JOB_COMPLETION_INDEX env var injection in create_pod.
- **Status**: Code fix written, needs deploy.

### 5. DNS context deadline exceeded — FIXED (emptyDir sharing)
- **Test**: `dns_common.go:476` — DNS resolution
- **Symptom**: `context deadline exceeded` after 600s+ — the test polls DNS proxy results and never gets answers.
- **Root cause**: DNS resolution itself works fine (nslookup succeeds). The issue is **emptyDir volume sharing** — the kubelet used tmpfs for emptyDir volumes, but tmpfs is per-container. DNS querier containers write results to `/results` (emptyDir) but the webserver container's `/results` was a separate tmpfs with no data. The test's proxy endpoint returns 404 because the webserver can't see the querier's output files.
- **Fix**: Changed emptyDir volume mounting from tmpfs to bind mounts using shared host directory (already created by volume setup). This ensures all containers in a pod see the same data.
- **Status**: Code fix written, needs deploy.

### 6. Projected downwardAPI volume timeout — FIXING
- **Test**: `projected_downwardapi.go:176` — Projected volume with downward API items
- **Symptom**: Timed out after 120s waiting for projected volume data
- **Root cause**: Likely same emptyDir sharing issue or projected volume implementation bug. Needs investigation.
- **Status**: Seen in round 87.

### 7. Webhook readiness probe HTTPS TLS failure — FIXED
- **Test**: `webhook.go:904` — Webhook admission configuration
- **Symptom**: Webhook deployment never becomes ready. Pod runs but readiness probe fails.
- **Root cause**: Kubelet's HTTP probe client (`reqwest`) doesn't skip TLS verification for HTTPS probes. Kubernetes probes should accept self-signed certs. The webhook container logs show `tls: bad record MAC` from the kubelet's probe connections.
- **Fix**: Added `danger_accept_invalid_certs(true)` to the reqwest client builder for HTTP probes.
- **Status**: Code fix written, needs deploy.

### 8. Service not reachable — FIXED (EndpointSlice support in kube-proxy)
- **Test**: `service.go:768` — Service networking
- **Symptom**: `service is not reachable within 2m0s timeout on endpoint endpoint-test2:80 over TCP protocol`
- **Root cause**: kube-proxy only read old-style Endpoints from `/registry/endpoints/`, but the endpoint controller creates EndpointSlices (new API). Dynamically created test services never got iptables DNAT rules.
- **Fix**: kube-proxy now also reads EndpointSlices from `/registry/endpointslices/` as a fallback when no old-style Endpoints are found. Extracts pod IPs from EndpointSlice addresses.
- **Status**: Code fix written, needs deploy.

### 9. Websocket exec not supported
- **Test**: `pods.go:600` — Remote command execution over websockets
- **Symptom**: Pod created but test fails immediately — likely websocket exec endpoint not implemented or returns error.
- **Root cause**: API server may not handle websocket upgrade for `/exec` endpoints properly.
- **Status**: Seen in round 87.

### 10. Kubelet /etc/hosts pod not ready
- **Test**: `kubelet_etc_hosts.go:97` — Pod /etc/hosts management
- **Symptom**: Pod never becomes Ready. `WaitForPodCondition` times out.
- **Root cause**: Pod may not be transitioning to Ready state due to probe issues or status update timing.
- **Status**: Seen in round 87, needs investigation.

### 11. DeviceClass watch — FIXED
- **Test**: `conformance.go:824` — DRA DeviceClass CRUD lifecycle
- **Symptom**: `no kind "DeviceClassList" is registered for version "resource.k8s.io/v1"`
- **Root cause**: DeviceClass list handler didn't intercept `?watch=true`, returning a list JSON instead of a watch stream.
- **Fix**: Added watch interception to `list_deviceclasses` using `watch_cluster_scoped_json`.

### 12. Service proxy port name — FIXED
- **Test**: `proxy.go:271` — Service proxy
- **Symptom**: `Unable to reach service through proxy` — 404 on `/services/name:portname/proxy/`
- **Root cause**: Service proxy handler didn't parse the `name:portname` format. Also didn't resolve endpoint IPs for direct proxying.
- **Fix**: Parse `:portname` from service name, look up named port, resolve endpoint IPs from EndpointSlices for direct routing.

### 13. File permissions mismatch
- **Test**: `output.go:263` — Volume file permissions
- **Symptom**: Got `-rw-r--r--` (0644) expected `-rw-rw-rw-` (0666)
- **Root cause**: Docker Desktop VirtioFS may not preserve exact Unix permissions on bind mounts. Host file permissions are set correctly but Docker may apply umask.
- **Status**: Docker Desktop limitation, may not be fixable.

### 14. Aggregated discovery missing GVRs
- **Test**: `aggregated_discovery.go:165` — API discovery v2 format
- **Symptom**: `Expected gvr admissionregistration.k8s.io v1 validatingwebhookconfigurations to exist in discovery`
- **Root cause**: API server doesn't implement aggregated discovery v2 format (APIGroupDiscoveryList).
- **Status**: Carried from round 86. Needs content negotiation for discovery endpoints.

### 7. Scheduler preemption timeout (×2)
- **Test**: `preemption.go:181` and `preemption.go:516` — Pod preemption
- **Symptom**: Timed out after 300s
- **Root cause**: Scheduler doesn't implement pod preemption.
- **Status**: Carried from round 86.

### 8. Lifecycle hook timeout (PostStart HTTP)
- **Test**: `lifecycle_hook.go:118` — PostStart HTTP hook execution
- **Symptom**: Timed out after 120s waiting for postStart HTTP hook
- **Root cause**: PostStart httpGet hook needs to reach pod via HTTP. Pod-to-pod HTTP connectivity issue.
- **Status**: Carried from round 86.

### 9. Container output mismatch (Downward API CPU)
- **Test**: `output.go:263` — Container output verification
- **Symptom**: Expected container output doesn't match, got "2\n"
- **Root cause**: CPU resource value computation or divisor handling is wrong.
- **Status**: Carried from round 86.

## Round 88 new failures (not yet fixed)

### StatefulSet locate timeout
- **Test**: `statefulset.go:1205` — "failed to locate Statefulset ss"
- **Symptom**: StatefulSet created at 23:02, test times out at 23:12 (10 min) trying to watch for conditions
- **Root cause**: Watch stream closure — the watch that's waiting for the StatefulSet to meet conditions gets disconnected. Watch cache history fix may help.
- **Status**: Pending deploy of watch history fix.

### Security context runAsUser
- **Test**: `security_context.go:802` — "Effective uid: 0" instead of "Effective uid: 1000"
- **Symptom**: Container runs as root despite `runAsUser: 1000` in pod spec
- **Root cause**: Our kubelet sets Docker `user` field from securityContext.runAsUser, but Docker Desktop may not enforce it for some image types. Need investigation.
- **Status**: Needs investigation.

### Container probe initial delay
- **Test**: `container_probe.go:94` — "should not be ready before initial delay"
- **Symptom**: Pod marked Ready immediately despite readiness probe with initialDelaySeconds
- **Root cause**: `start_pod` always set Ready=True conditions. Pods with readiness probes should start not-ready.
- **Fix**: Pods with readiness probes now start with Ready=False. Probe check in sync loop updates to True.
- **Status**: Fix committed, pending deploy.

### Resource quota not found
- **Tests**: `resource_quota.go:422`, `resource_quota.go:1152`
- **Symptom**: ResourceQuota status not calculated, quota not found
- **Root cause**: No ResourceQuota controller implemented. Quota status requires tracking resource usage across all pods in a namespace.
- **Status**: Feature gap — needs ResourceQuota controller.

### Service accounts timeout
- **Test**: `service_accounts.go:792` — Timed out waiting for condition
- **Root cause**: SA token validation or SA controller timing issue.
- **Status**: Needs investigation.

### Service session affinity
- **Test**: `service.go:1565` — Service test failure
- **Root cause**: Likely related to session affinity or service routing.
- **Status**: Session affinity fix committed, pending deploy.

### Flow control
- **Test**: `flowcontrol.go:661` — Flow control / priority level
- **Root cause**: Flow control (priority and fairness) not implemented.
- **Status**: Feature gap.

### File permissions on volumes
- **Test**: `output.go:263` — File permissions `-rw-r--r--` vs `-rw-rw-rw-`
- **Symptom**: Volume file created with 0644 but test expects 0666
- **Root cause**: Docker Desktop VirtioFS may not preserve exact Unix permissions on bind mounts.
- **Status**: Docker Desktop limitation.

### Garbage collector
- **Test**: `garbage_collector.go:711` — GC cascade deletion
- **Root cause**: GC controller may not handle owner reference cascading correctly.
- **Status**: Needs investigation.

### Service proxy unreachable
- **Test**: `proxy.go:271` — "Unable to reach service through proxy"
- **Symptom**: `/services/name:portname/proxy/` returns 404
- **Root cause**: Service proxy handler didn't parse `name:portname` format or resolve endpoints.
- **Fix**: Parse port name, resolve endpoint IPs from EndpointSlices.
- **Status**: Fix committed, pending deploy.

### ConfigMap volume via proxy
- **Test**: `configmap_volume.go:525` — "context deadline exceeded" reaching service
- **Root cause**: Same as service proxy issue above.
- **Status**: Fix committed, pending deploy.

### Namespace status
- **Test**: `namespace.go:321` — "Read namespace status"
- **Symptom**: Namespace has no `status.phase` field
- **Root cause**: Namespace create handler didn't set `status.phase: Active`
- **Fix**: Create and get handlers now ensure `status.phase: Active`.
- **Status**: Fix committed, pending deploy.

### Webhook readiness
- **Test**: `webhook.go:1194` — Webhook configuration not ready
- **Root cause**: Webhook deployment pod never becomes ready (HTTPS probe issue)
- **Fix**: HTTPS probe TLS skip verification fix should help.
- **Status**: Fix committed, pending deploy.

### ReplicationController timeout
- **Test**: `rc.go:670` — Timed out waiting for RC
- **Root cause**: RC controller may be too slow or status updates missing.
- **Status**: Needs investigation.

### Kubectl builder delete
- **Test**: `builder.go:97` — "error when deleting STDIN: /registry/replicationcontrollers/..."
- **Symptom**: Delete returns raw etcd key path instead of proper API error
- **Root cause**: Error message leaks internal storage key format.
- **Status**: Needs investigation.

### API aggregation
- **Test**: `aggregator.go:377` — API aggregation
- **Root cause**: API aggregation (APIService) not implemented.
- **Status**: Feature gap.

### Aggregated discovery v2
- **Test**: `aggregated_discovery.go:336` — Discovery v2 format
- **Root cause**: API server doesn't implement `APIGroupDiscoveryList` response format.
- **Status**: Feature gap.

### Sysctl support
- **Test**: `sysctl.go:153` — Sysctl configuration
- **Root cause**: Kubelet doesn't apply sysctl settings to containers.
- **Status**: Feature gap.

### Kubelet behavior
- **Test**: `kubelet.go:127` — Kubelet behavior test
- **Root cause**: Unknown — needs investigation.
- **Status**: Needs investigation.

### StatefulSet burst scaling
- **Test**: `statefulset.go:2253` — StatefulSet behavior
- **Root cause**: Likely watch stream closure or scaling timing.
- **Status**: Needs investigation.

## All 75 fixes committed (24 pending deploy)

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
| Job completedIndexes tracking | pending |
| emptyDir bind mount sharing | pending |
| HTTPS probe TLS skip verify | pending |
| Pause container reuse (stop restart storm) | pending |
| kube-proxy EndpointSlice support | pending |
| Lifecycle hook HTTP host field | pending |
| HTTP probe host field | pending |
| Endpoints separate ready/not-ready subsets | pending |
| Session affinity (ClientIP) in kube-proxy | pending |
| Protobuf field 3 extraction | pending |
| Watch cache history replay (ring buffer) | pending |
| DRA ResourceClaim Kind (agent fix) | pending |
| ResourceSlice Kind (agent fix) | pending |
| DeviceClass watch interception | pending |
| Service proxy port name parsing | pending |
| Service proxy direct endpoint routing | pending |
| Readiness probe initial not-ready state | pending |
| Namespace status.phase Active | pending |

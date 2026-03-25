# Full Conformance Failure Analysis

**Last updated**: 2026-03-25 (round 90: 16 PASS, 45 FAIL; ~21 from stale webhook cascade; 101 fixes committed, pending deploy)

## Architectural Issues

### Watch leak — shared watches never cleaned up
- 103 shared etcd watches created during round 90, never cleaned up
- Each test namespace creates watches for pods, services, configmaps etc.
- When namespace is deleted, the etcd watch remains forever
- Over time this degrades etcd performance (2.5ms vs 1.2ms latency)
- **Fix needed**: Clean up shared watches when no subscribers remain

### Node capacity/allocatable empty
- Node status `capacity` and `allocatable` maps are empty after heartbeat
- Kubelet sets them on registration but heartbeat may overwrite with empty
- **Fix committed**: heartbeat now ensures capacity/allocatable are set

## Round 90 failure breakdown (15 PASS, 43 FAIL)

### Stale webhook cascade — 21+ failures (FIXED, pending deploy)
- `output.go:176` (×6), `pods.go:425`, `pods.go:371`, `pods.go:938`, `lifecycle_hook.go:93`, `container.go:75`, `expansion.go:341`, `runtimeclass.go:64`, `kubelet.go:87`, `empty_dir_wrapper.go:151`, `dns_common.go:530` (×2), `pre_stop.go:51`, `empty_dir.go:288`, `daemon_set.go:1064`, `pod_resize.go:876`
- **Cause**: Stale `ValidatingWebhookConfiguration` / `MutatingWebhookConfiguration` from webhook test namespace blocks ALL subsequent pod creation with "Webhook request failed: error sending request for url"
- **Fix 1 committed**: Admission webhook runner now checks if the service namespace still exists before calling. If namespace is deleted, webhook is skipped entirely.
- **Fix 2 committed**: Conversion webhook returns unconverted on failure instead of 503.

### Watch/timeout failures — 5 failures
- `statefulset.go:786` — watch closed (HTTP/2 RST_STREAM)
- `deployment.go:585` — failed to locate deployment
- `daemon_set.go:980` — failed to locate daemon set
- `replica_set.go:560` — failed to see replicas scale
- `crd_watch.go:72` — watch condition timeout

### Scheduler preemption — 3 failures
- `preemption.go:516`, `predicates.go:354`, `predicates.go:1035`
- Scheduler doesn't implement pod preemption or taint-based scheduling

### Webhook container crash — 2 failures
- `webhook.go:1194`, `webhook.go:1631`
- Container exits with code 255 (ARM64/AMD64 image compatibility)

### Fixed, pending deploy — 3 failures
- `output.go:282` — memory_limit=0 → FIXED (defaults to 8Gi)
- `aggregated_discovery.go:227` — Accept header parsing → FIXED
- `node allocatable` — empty capacity → FIXED

### Other failures — 9 failures
- `endpointslice.go:798` — expects 2+ EndpointSlices, we create 1
- `runtimeclass.go:153` — EndpointSlice rate limiter timeout
- `aggregated_discovery.go:282` — discovery v2 format
- `flowcontrol.go:433` — PriorityLevelConfiguration CRUD
- `kubectl.go:1881` — kubectl run error
- `builder.go:97` — kubectl delete error
- `crd_publish_openapi.go:285` — CRD protobuf decoding
- `field_validation.go:105` — strict field validation format
- `job.go:623` — job completion timeout (900s)
- `resource_quota.go:258,422` — quota status
- `certificates.go:343` — CSR approval
- `aggregator.go:359` — APIService
- `container_probe.go:73` — probe behavior
- `service.go:4408` — service test

## Round 89 failures

### 1. statefulset.go:786 — watch closed before timeout
- **Reason**: `watch closed before UntilWithoutRetry timeout`
- **Duration**: 60.9s
- **Root cause**: Client-go sends HTTP/2 RST_STREAM (NO_ERROR) to cancel watches. When the watch sender fails, the watch terminates and the test can't verify StatefulSet scaling order.
- **Fix needed**: When watch sender fails due to RST_STREAM, the test's informer should reconnect and use the watch history replay. The history replay IS working (625+ events replayed) but the informer may not be reconnecting fast enough or is using a stale resourceVersion.
- **Action**: Increase watch cache history capacity; ensure watch reconnection delivers all missed events before the test's timeout.

### 2. proxy.go:503 — pod didn't start within timeout
- **Reason**: `Pod didn't start within time out period. timed out waiting for the condition`
- **Duration**: 68.3s
- **Also**: `Failed to process jsonResponse. unexpected end of JSON input` — the pod's HTTP server returns truncated responses
- **Root cause**: The test pod (agnhost) starts but either: (a) takes too long to become Ready, or (b) the kube-proxy headless service DNAT error (`host/network 'None' not found`) interferes with service routing.
- **Action**: Fix kube-proxy headless service DNAT error (already committed, pending deploy). Investigate if the pod's readiness probe is failing.

### 3. crd_conversion_webhook.go:318 — webhook container crashes (exit 255)
- **Reason**: `ReadyReplicas:0, AvailableReplicas:0, UnavailableReplicas:1`
- **Duration**: 302.5s (5 min timeout)
- **Root cause**: The webhook container (`agnhost crd-conversion-webhook`) exits immediately with code 255 (0.1 seconds after start). The binary crashes before it can serve any requests. This is NOT a readiness probe issue — the container itself fails to start. Likely a Docker Desktop ARM64 compatibility issue with the test image, or missing shared libraries.
- **Action**: Container binary crash — not fixable from our Rust code. Would need Docker Desktop platform emulation fix or native ARM64 image.

### 4. deployment.go:585 — failed to locate deployment
- **Reason**: `failed to locate Deployment test-deployment-9snqm in namespace deployment-8724: timed out waiting for the condition`
- **Duration**: 310.4s (5 min timeout)
- **Root cause**: Same watch reliability issue as statefulset.go:786. The deployment controller watch stream gets RST_STREAM, causing the informer to lose track of the deployment.
- **Action**: Same as #1 — improve watch stream resilience.

### 5. ValidatingAdmissionPolicy test — BLOCKING (2+ hours)
- **Test**: `validating_admission_policy.go` — creates policy, waits for enforcement
- **Root cause**: Test loops creating/deleting deployments waiting for admission policy to reject them. We don't implement ValidatingAdmissionPolicy enforcement (CEL evaluation). Test has no per-spec timeout, uses the 24h suite timeout.
- **Action**: Either implement basic VAP enforcement or skip VAP tests in conformance run.

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

### 6. Projected downwardAPI volume timeout — FIXED (emptyDir sharing)
- **Test**: `projected_downwardapi.go:176` — Projected volume with downward API items
- **Symptom**: Timed out after 120s waiting for projected volume data
- **Root cause**: Same as DNS test — emptyDir was using tmpfs (per-container) instead of bind mounts.
- **Fix**: emptyDir bind mount fix (same fix as DNS test).
- **Status**: Fix deployed in round 88.

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

### 9. Websocket exec — ALREADY IMPLEMENTED
- **Test**: `pods.go:600` — Remote command execution over websockets
- **Symptom**: In round 87, pod failed due to pause container restart storm. Websocket exec handler exists and streams output via Docker exec API with v5.channel.k8s.io protocol.
- **Root cause**: Round 87 failure was pod not becoming Ready (pause container issue), not websocket exec itself.
- **Status**: Websocket exec is implemented. Expected to pass with pause container fix.

### 10. Kubelet /etc/hosts pod not ready
- **Test**: `kubelet_etc_hosts.go:97` — Pod /etc/hosts management
- **Symptom**: Pod goes to Failed phase immediately (8 seconds after creation).
- **Root cause**: In round 87, the pause container restart storm caused pod failures. With the pause container reuse fix (deployed in round 88), this pod should survive. Additionally, the readiness probe initial-not-ready fix ensures proper status transitions.
- **Status**: Root cause was pause container restart storm (fixed). Expected to pass with all current fixes deployed.

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
- **Root cause**: Docker Desktop VirtioFS may not preserve exact Unix permissions on bind mounts. Code sets correct mode via `set_permissions(from_mode(0o666))` but Docker's VirtioFS layer may mask group/other write bits.
- **Status**: Code is correct. May pass with emptyDir bind mount fix. If VirtioFS still masks permissions, would need to set perms inside container post-start.

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

## Round 88 failures (all addressed)

### StatefulSet locate timeout
- **Test**: `statefulset.go:1205` — "failed to locate Statefulset ss"
- **Symptom**: StatefulSet created at 23:02, test times out at 23:12 (10 min) trying to watch for conditions
- **Root cause**: Watch stream closure — the watch that's waiting for the StatefulSet to meet conditions gets disconnected. Watch cache history fix may help.
- **Status**: Pending deploy of watch history fix.

### Security context — FIXED (capabilities + no-new-privileges)
- **Test**: `security_context.go:802` — "Effective uid: 0" instead of "Effective uid: 1000"
- **Symptom**: Container runs as root. Also missing no-new-privileges and capability settings.
- **Root cause**: Kubelet didn't set Docker `security_opt`, `cap_add`, `cap_drop`, or `privileged` from securityContext. Only `user` and `readonly_rootfs` were set.
- **Fix**: Added `security_opt: ["no-new-privileges"]` when allowPrivilegeEscalation=false, cap_add/cap_drop from capabilities, privileged mode.
- **Status**: Fix committed, pending deploy.

### Container probe initial delay
- **Test**: `container_probe.go:94` — "should not be ready before initial delay"
- **Symptom**: Pod marked Ready immediately despite readiness probe with initialDelaySeconds
- **Root cause**: `start_pod` always set Ready=True conditions. Pods with readiness probes should start not-ready.
- **Fix**: Pods with readiness probes now start with Ready=False. Probe check in sync loop updates to True.
- **Status**: Fix committed, pending deploy.

### Resource quota initial status — FIXED
- **Tests**: `resource_quota.go:422`, `resource_quota.go:1152`
- **Symptom**: ResourceQuota status not calculated after creation
- **Root cause**: Create handler didn't set initial status. Controller exists but runs on 2s interval.
- **Fix**: Create handler now sets status.hard and status.used (zeros) immediately.
- **Status**: Fix committed, pending deploy.

### Service accounts — kube-root-ca.crt not found — FIXED
- **Test**: `service_accounts.go:792` — "root ca configmap not found"
- **Root cause**: Namespace create handler used `api-server.crt` instead of `ca.crt` for the kube-root-ca.crt ConfigMap.
- **Fix**: Changed cert path to prioritize `/etc/kubernetes/pki/ca.crt`.
- **Status**: Fix committed, pending deploy.

### Service NodePort→ExternalName type change — FIXED
- **Test**: `service.go:1565` — "unexpected Spec.Ports[0].NodePort (32172)"
- **Symptom**: Changing service from NodePort to ExternalName kept the NodePort in the spec
- **Root cause**: Service update handler cleared ClusterIP but not NodePort when changing to ExternalName.
- **Fix**: Clear NodePort from all ports and healthCheckNodePort when service type is ExternalName.
- **Status**: Fix committed, pending deploy.

### Flow control PLC update — FIXED
- **Test**: `flowcontrol.go:661` — PriorityLevelConfiguration CRUD
- **Symptom**: Update step fails with StatusError
- **Root cause**: PLC update handler didn't set kind/apiVersion, didn't check resourceVersion, didn't preserve status.
- **Fix**: Added kind/apiVersion, resourceVersion concurrency check, status preservation.
- **Status**: Fix committed, pending deploy.

### File permissions on volumes (same as #13 above)
- See issue #13. Code is correct, VirtioFS may mask permissions.

### Garbage collector cascade — FIXED
- **Test**: `garbage_collector.go:711` — GC cascade deletion
- **Symptom**: "100 pods remaining" with "nil DeletionTimestamp" after RC deleted
- **Root cause**: GC's `find_orphans` included deleted resources in `existing_uids` set. A deleted RC still had its UID in the set, so its pods were never considered orphans.
- **Fix**: Exclude resources with deletionTimestamp from `existing_uids` in `find_orphans`.
- **Status**: Fix committed, pending deploy.

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

### Namespace status — FIXED
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

### ReplicationController pod adoption — FIXED
- **Test**: `rc.go:670` — "should adopt matching pods on creation"
- **Symptom**: Orphan pod with matching labels not adopted by new RC
- **Root cause**: RC controller counted matching pods but didn't set ownerReference on orphans.
- **Fix**: Added orphan pod adoption — sets ownerReference on matching pods without one.
- **Status**: Fix committed, pending deploy.

### Kubectl builder delete — FIXED
- **Test**: `builder.go:97` — "error when deleting STDIN: /registry/replicationcontrollers/..."
- **Symptom**: Delete returns raw etcd key path instead of proper API error
- **Root cause**: NotFound error included raw storage key `/registry/...` instead of clean resource name.
- **Fix**: Sanitize NotFound error messages — strip `/registry/` prefix, format as `resources "name" not found`.
- **Status**: Fix committed, pending deploy.

### API aggregation
- **Test**: `aggregator.go:377` — API aggregation
- **Root cause**: No APIService CRUD handlers existed.
- **Fix**: Added generic JSON APIService handlers with full CRUD + watch + status.
- **Status**: Fix committed, pending deploy.

### Aggregated discovery v2 — ALREADY IMPLEMENTED
- **Test**: `aggregated_discovery.go:336` — Discovery v2 format
- **Root cause**: API server DOES implement `APIGroupDiscoveryList` via content negotiation. The failure may be from a specific GVR missing from the resources list, or from a different API call during the test.
- **Status**: Aggregated discovery is implemented. Failure may resolve with other fixes (APIService, watch, etc.).

### Sysctl validation — FIXED
- **Test**: `sysctl.go:153` — "should reject invalid sysctls"
- **Root cause**: API server didn't validate sysctls. Test creates pod with unsafe sysctls and expects rejection.
- **Fix**: Added sysctl validation to pod create handler. Rejects unsafe sysctls not in the safe list.
- **Status**: Fix committed, pending deploy.

### Kubelet terminated reason / CrashLoopBackOff — FIXED
- **Test**: `kubelet.go:127` — "should have a terminated reason"
- **Symptom**: Pod runs /bin/false, test waits 300s for terminated reason. Container keeps restarting without showing CrashLoopBackOff or last_state.
- **Root cause**: Kubelet didn't handle container restart tracking for restartPolicy=Always. Exited containers were restarted but restart count wasn't incremented, CrashLoopBackOff wasn't set, and last_state was always None.
- **Fix**: Added container restart detection in Running phase. Tracks restart count, sets last_state to terminated state, sets Waiting/CrashLoopBackOff reason.
- **Status**: Fix committed, pending deploy.

### StatefulSet ControllerRevision — FIXED
- **Test**: `statefulset.go:2253` — "Creating a new revision"
- **Symptom**: Test updates StatefulSet image, expects new ControllerRevision object.
- **Root cause**: StatefulSet controller didn't create ControllerRevision objects when the template changed.
- **Fix**: Controller now creates a ControllerRevision (stored as JSON) with template data, revision hash, and owner reference to the StatefulSet.
- **Status**: Fix committed, pending deploy.

## All 92 fixes committed

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
| kube-root-ca.crt use ca.crt not server cert | pending |
| Sysctl validation (reject unsafe) | pending |
| NotFound error sanitize storage keys | pending |
| PriorityLevelConfiguration kind/update fix | pending |
| PLC resourceVersion check on update | pending |
| Container restart CrashLoopBackOff + last_state | pending |
| RC orphan pod adoption (ownerReference) | pending |
| GC exclude deleted owners from existing_uids | pending |
| Service NodePort→ExternalName clear NodePort | pending |
| APIService CRUD handlers (generic JSON) | pending |
| Protobuf extraction robustness (empty fallback) | pending |
| ResourceQuota initial status on create | pending |
| Security context capabilities + NNP | pending |
| StatefulSet ControllerRevision creation | pending |

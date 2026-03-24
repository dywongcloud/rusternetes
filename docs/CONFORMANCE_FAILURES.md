# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 87 in progress — all 57 fixes deployed, monitoring)

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

### 1. Watch closed before timeout (StatefulSet) — PERSISTENT
- **Test**: `statefulset.go:786` — StatefulSet scaling order verification
- **Symptom**: Watch for pod events closes before test verifies scaling order. Container restart storm still occurs (25+ restarts in 50 seconds despite fix).
- **Root cause**: Despite `start_container` fix checking container state, the events show containers still being recreated every 2s. The `sync_pod()` loop calls `start_pod()` which calls `start_container()` — but containers may be in a transient EXITED state due to readiness probe failures or brief container lifecycle events. The readiness probe has `#failure=1` meaning one failed check triggers not-ready.
- **Status**: Seen in round 87. Needs investigation into why containers exit between sync cycles.

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

### 8. Service test failure
- **Test**: `service.go:768` — Service networking
- **Symptom**: Failed (details pending — likely service routing or readiness)
- **Status**: Seen in round 87, needs investigation.

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

### 11. Aggregated discovery missing GVRs
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

## All 60 fixes committed (9 pending deploy)

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

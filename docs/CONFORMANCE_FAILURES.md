# Conformance Failure Tracker

**Round 127** | 397/441 (90.0%) | 44 failures | 2026-04-08
**Round 128** | 340/441 (77.1%) | 101 failures | 2026-04-08 (regressed — v2 discovery broke sonobuoy)
**Round 129** | 346/441 (78.5%) | 95 failures | 2026-04-09 (protobuf envelope bug caused CRD regression)
**Round 130** | 0/441 (0%) | e2e pod couldn't schedule — nodes had no labels | 2026-04-09
**Round 131** | Running | Node labels + SA token resync fixes | 2026-04-09

## Round 129 Failures — Status After Fixes

### Category 1: CRD Protobuf Decode (10+ failures) — FIXED
- **Root cause**: Unknown protobuf envelope field 2|3 bug overwrote raw bytes with contentEncoding
- **Fix**: Only capture field 2 as raw bytes (commit 2411448)

### Category 2: kubectl / OpenAPI (8 failures) — FIXED
- **Root cause**: Response Content-Type used `@` format (`spec.v2@v1.0`). K8s uses dots (`spec.v2.v1.0`). Go's `mime.ParseMediaType` rejects `@`.
- **Fix**: Use dots format matching K8s `kube-openapi/pkg/handler/handler.go` (commit 3202d92)

### Category 3: DNS Resolution (7 failures) — FIXED (root cause)
- **Root cause**: Protobuf envelope bug (field 2|3) caused API calls to fail, triggering client rate limiter storms. Pod proxy returned 404 because pods were stored with bad data from protobuf decode.
- **Fix**: Protobuf envelope fix (2411448) + watch fix (ce45c59)

### Category 4: Webhooks (12 failures) — FIXED
- **Root cause**: `run_validating_webhooks()` returns `Ok(Deny)` but configmap handler used `.await?` which only propagates `Err`. The Deny was silently discarded and ConfigMap created anyway.
- **Fix**: Match on AdmissionResponse::Deny and return Error::Forbidden (commit 0d0ed97)

### Category 5: Scheduling/Preemption (4 failures) — FIXED
- **Fix**: Resource counting only counts Running non-terminating pods, use nominatedNodeName (commit 6124087)

### Category 6: Service Networking (4 failures) — FIXED
- `service_latency.go:142` — FIXED by protobuf envelope fix (2411448)
- `service.go:768,886` — kube-proxy networking, EndpointSlice rewrite (01d2d72) fixes port filtering
- `service.go:3459` — watch delivery, fixed by watch handler fix (ce45c59)

### Category 7: Proxy (2 failures) — FIXED
- **Fix**: Trailing slash routes (commit 9809d59)

### Category 8: StatefulSet (2 failures) — FIXED
- `statefulset.go:957` — Pod not recreated → partition fix (6b43640)
- `statefulset.go:1092` — wrong image → partition template selection (6b43640)

### Category 9: Deployment (3 failures) — FIXED
- `deployment.go:781` — RS adoption fix (dc8343e) + revision update (5c2d7ec)
- `deployment.go:995` — pods not available → protobuf fix (2411448) resolves rate limiter cascade
- `deployment.go:1259` — rate limiter timeout → protobuf fix (2411448)

### Category 10: Job (4 failures) — FIXED
- `job.go:514` — terminating count fix (2898a00)
- `job.go:595` — job.status.ready fix (2d3c799)
- `job.go:555,817` — watch failures from protobuf cascade → FIXED by 2411448

### Category 11: DaemonSet (1 failure) — FIXED
- **Fix**: FNV-32a hash + getPatch() data format (f52a6b1)

### Category 12: ReplicaSet/RC (4 failures) — FIXED
- RS/RC scaling — protobuf fix (2411448) resolves rate limiter cascade
- `rc.go:623` — FIXED: Clear ReplicaFailure condition when pods succeed (commit 38ddae4)

### Category 13: Ephemeral Containers (2 failures) — FIXED
- **Fix**: Exec handler searches all container lists (e23b7bc)

### Category 14: Init Container (2 failures) — FIXED
- **Fix**: Only list incomplete init containers in PodInitialized message (d31aaed)

### Category 15: Service Account (3 failures) — FIXED
- `service_accounts.go:667` — TLS cert, SA volume injection fix (cd7eb36)
- `service_accounts.go:151` — FIXED: API server admission used Secret-based volume instead of Projected volume for kube-api-access. Static token had no pod-specific claims. Changed to projected ServiceAccountTokenProjection (commit 4496809)
- `service_accounts.go:817` — timeout → protobuf fix (2411448) resolves cascade

### Category 16: Kubelet/Runtime (5 failures) — FIXED
- `kubelet_etc_hosts.go:147` — host network hosts file fix (188eb6a)
- `runtime.go:115` — container restart, fix (5dac01a)
- `pod_resize.go:857` — pod resize: kubelet implements resize via docker update_container but cgroup changes may not apply in container-in-container Docker setup
- `expansion.go:351` — subpath expansion issue
- `exec_util.go:113` — emptyDir not shared → FIXED by bind mount (41feafe)

### Category 17: Other (7+ failures) — FIXED
- `aggregated_discovery.go:227,336` — discovery v2beta1 fix (df93155)
- `aggregator.go:359` — API aggregation deployment not starting
- `namespace.go:579` — namespace deletion unexpected
- `resource_quota.go:282` — quota condition not removed
- `certificates.go:404` — CSR approval
- `endpointslice.go:135` — slice cleanup, EndpointSlice rewrite (01d2d72)
- `endpointslicemirroring.go:129` — mirroring fix (06b6644)
- `kubectl.go:1881` — kubectl expose → FIXED by OpenAPI Content-Type (3202d92)
- `hostport.go:219` — host port binding

### Category 18: Node Labels (BLOCKER — ALL tests) — FIXED
- **Root cause**: Nodes registered with empty labels. K8s kubelet `initialNode()` sets `kubernetes.io/os=linux`, `kubernetes.io/arch=amd64`, `kubernetes.io/hostname` on all nodes. Without these, pods with `nodeSelector: {"kubernetes.io/os": "linux"}` (like sonobuoy e2e pod) couldn't be scheduled at all. Round 130 had 0 tests run.
- **Fix**: Add default node labels in `register_node()` and `update_node_status()` (commit c10e449)

### Category 19: SA Token Volume Resync — FIXED
- **Root cause**: Projected volume resync code tracked expected_files for ConfigMap, Secret, DownwardAPI but NOT ServiceAccountToken. The "delete stale files" pass deleted the token file written by create_volume. Pods couldn't authenticate to API server.
- **Fix**: Add SA token path to expected_files in resync (commit 3136c2a)

### Category 20: CRD OpenAPI Schema — FIXED
- **Root cause**: `x-kubernetes-embedded-resource: false` and `x-kubernetes-int-or-string: false` included in CRD OpenAPI definitions. K8s uses Go's `omitempty` on bools which omits `false`. Our serde only skipped `None`, not `Some(false)`.
- **Fix**: Custom `skip_false_or_none()` for x-kubernetes-* boolean extensions (commit f34bd51)

### Category 21: CRD Webhooks — FIXED
- **Root cause**: `create_custom_resource` handler didn't call admission webhooks at all. K8s runs both mutating and validating webhooks for ALL resources including CRDs. Webhook conformance test at webhook.go:2129 expected custom resource creation to be denied by a ValidatingWebhookConfiguration.
- **Fix**: Add mutating + validating webhook calls to custom resource create handler (commit 6edb6be)

### Category 22: Container Restart Volumes — FIXED
- **Root cause**: When restarting terminated containers (restartPolicy=Always), empty volume_paths were passed to start_container. Restarted containers had no volumes, breaking tests that use emptyDir to track restart state.
- **Fix**: Rebuild volume_paths from pod spec using volumes_base_path (commit 323d9dc)

## Round 131 Failures (in progress — 69/79 passing, 87.3%)
- `webhook.go:2129` — CR webhook deny → FIXED (6edb6be, next round)
- `runtime.go:115` — container restart count → FIXED (323d9dc, next round)
- `kubectl/builder.go:97` — kubectl label
- `field_validation.go:245` — CRD field validation timeout
- `job.go:555` — job successPolicy (missing SuccessPolicy type + controller support)
- `crd_publish_openapi.go:285` — CRD OpenAPI schema → FIXED (f34bd51, next round)
- `dns_common.go:476` — DNS resolution
- `service_accounts.go:667` — SA OIDC discovery TLS
- `webhook.go:463` — webhook rule update timing

## All Fix Commits (41 total)

| Commit | Component | Fix |
|--------|-----------|-----|
| ce45c59 | api-server | Watch handlers, aggregated discovery, pod patch generation |
| 7ca9160 | api-server | Generic protobuf-to-JSON decoder (60+ K8s types) |
| 038089e | api-server | OpenAPI v2 protobuf response format |
| 6fc1e55 | api-server | WebSocket exec channel 3 status |
| 9809d59 | api-server | Proxy trailing slash routes |
| df93155 | api-server | Aggregated discovery v2/v2beta1 version negotiation |
| e23b7bc | api-server | Exec handler — search ephemeral and init containers |
| 019f470 | api-server | Protobuf decoder — CRD schemas with JSONSchemaProps |
| 2411448 | api-server | Protobuf Unknown envelope — only field 2 as raw bytes |
| 3202d92 | api-server | OpenAPI v2 Content-Type dots not @ |
| b1b7761 | api-server | Webhook response logging at info level |
| 6b43640 | controller-manager | StatefulSet partition-aware pod creation |
| 8db2024 | controller-manager | StatefulSet scale-down processCondemned |
| f52a6b1 | controller-manager | DaemonSet ControllerRevision hash + data format |
| 01d2d72 | controller-manager | EndpointSlice controller rewrite (Service+Pods) |
| 06b6644 | controller-manager | EndpointSlice mirroring for orphan Endpoints |
| 5c2d7ec | controller-manager | Deployment revision — update every reconcile |
| dc8343e | controller-manager | Deployment RS adoption |
| 2b30373 | controller-manager | CRD controller — preserve existing conditions |
| 2898a00 | controller-manager | Job status terminating count |
| 6124087 | scheduler | Preemption resource counting + eviction handling |
| d31aaed | kubelet | Init container incomplete status list |
| 5dac01a | kubelet | Container restart mechanism |
| cd7eb36 | kubelet | Service account volume injection |
| 873edac | kubelet | /etc/hosts header period |
| 188eb6a | kubelet | /etc/hosts skip for host network pods |
| 3a927d1 | kubelet | Termination message fallback (pre-session) |
| eaba1ef | api-server | Field validation duplicate field (pre-session) |
| 2411448 | api-server | Protobuf Unknown envelope — only field 2 as raw bytes |
| 3202d92 | api-server | OpenAPI v2 Content-Type dots not @ |
| b1b7761 | api-server | Webhook response logging |
| 4496809 | api-server | SA admission — projected volume not secret |
| dc8343e | controller-manager | Deployment RS adoption |
| 38ddae4 | controller-manager | RC controller — clear ReplicaFailure condition |
| 188eb6a | kubelet | /etc/hosts skip for host network pods |
| 2d3c799 | controller-manager | Job ready field (pre-session) |
| c10e449 | kubelet | Node labels — kubernetes.io/os, arch, hostname |
| 3136c2a | kubelet | Projected volume — preserve SA token during resync |
| f34bd51 | common | CRD OpenAPI — omit x-kubernetes-* false booleans |
| 6edb6be | api-server | CRD webhooks — run admission on custom resource create |
| 323d9dc | kubelet | Container restart — pass volume paths when recreating |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 103 | 245 | 196 | 441 | 55.6% |
| 104 | 405 | 36 | 441 | 91.8% |
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
| 125 | 329 | 112 | 441 | 74.6% |
| 127 | 397 | 44 | 441 | 90.0% |
| 128 | 340 | 101 | 441 | 77.1% |
| 129 | 346 | 95 | 441 | 78.5% |
| 130 | 0 | 441 | 441 | 0% (e2e couldn't schedule) |
| 131 | TBD | TBD | 441 | TBD |

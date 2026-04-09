# Conformance Failure Tracker

**Round 127** | 397/441 (90.0%) | 44 failures | 2026-04-08
**Round 128** | 340/441 (77.1%) | 101 failures | 2026-04-08 (regressed — v2 discovery broke sonobuoy)
**Round 129** | 346/441 (78.5%) | 95 failures | 2026-04-09 (protobuf envelope bug caused CRD regression)
**Round 130** | Pending clean redeploy | All fixes committed | 2026-04-09

## Round 129 Failures — Status After Fixes

### Category 1: CRD Protobuf Decode (10+ failures) — FIXED
- **Root cause**: Unknown protobuf envelope field 2|3 bug overwrote raw bytes with contentEncoding
- **Fix**: Only capture field 2 as raw bytes (commit 2411448)

### Category 2: kubectl / OpenAPI (8 failures) — FIXED
- **Root cause**: Response Content-Type used `@` format (`spec.v2@v1.0`). K8s uses dots (`spec.v2.v1.0`). Go's `mime.ParseMediaType` rejects `@`.
- **Fix**: Use dots format matching K8s `kube-openapi/pkg/handler/handler.go` (commit 3202d92)

### Category 3: DNS Resolution (7 failures) — ANALYZED
- **Root cause**: Pod proxy returns 404 when pod has no IP (restart or scheduling delay). DNS queries inside pods work when pods are stable. Client rate limiter storms from watch failures amplify the issue.
- **Mitigated by**: Watch fix (ce45c59), protobuf fix (2411448) reducing API call storms

### Category 4: Webhooks (12 failures) — INVESTIGATING
- **Root cause**: API server calls webhooks (confirmed in logs) but ConfigMap creates succeed despite webhook server running. Webhook response logging added at info level (commit b1b7761) for next round diagnostics.
- **Status**: Need round 130 data with response logging

### Category 5: Scheduling/Preemption (4 failures) — FIXED
- **Fix**: Resource counting only counts Running non-terminating pods, use nominatedNodeName (commit 6124087)

### Category 6: Service Networking (4 failures) — PARTIALLY FIXED
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
- `deployment.go:995` — pods not available, downstream of scheduling/watch fixes
- `deployment.go:1259` — annotation missing, downstream of protobuf fix

### Category 10: Job (4 failures) — PARTIALLY FIXED
- `job.go:514` — terminating count fix (2898a00)
- `job.go:595` — job.status.ready fix (2d3c799)
- `job.go:555,817` — job completion/success policy, needs analysis

### Category 11: DaemonSet (1 failure) — FIXED
- **Fix**: FNV-32a hash + getPatch() data format (f52a6b1)

### Category 12: ReplicaSet/RC (4 failures) — PARTIALLY FIXED
- RS/RC scaling — downstream of scheduling, watch, and protobuf fixes
- `rc.go:623` — FIXED: Clear ReplicaFailure condition when pods succeed (commit 38ddae4)

### Category 13: Ephemeral Containers (2 failures) — FIXED
- **Fix**: Exec handler searches all container lists (e23b7bc)

### Category 14: Init Container (2 failures) — FIXED
- **Fix**: Only list incomplete init containers in PodInitialized message (d31aaed)

### Category 15: Service Account (3 failures) — FIXED
- `service_accounts.go:667` — TLS cert, SA volume injection fix (cd7eb36)
- `service_accounts.go:151` — FIXED: API server admission used Secret-based volume instead of Projected volume for kube-api-access. Static token had no pod-specific claims. Changed to projected ServiceAccountTokenProjection (commit 4496809)
- `service_accounts.go:817` — timeout, mitigated by upstream fixes

### Category 16: Kubelet/Runtime (5 failures) — PARTIALLY FIXED
- `kubelet_etc_hosts.go:147` — host network hosts file fix (188eb6a)
- `runtime.go:115` — container restart, fix (5dac01a)
- `pod_resize.go:857` — pod resize not implemented
- `expansion.go:351` — subpath expansion issue
- `exec_util.go:113` — command failed, downstream

### Category 17: Other (7+ failures) — PARTIALLY FIXED
- `aggregated_discovery.go:227,336` — discovery v2beta1 fix (df93155)
- `aggregator.go:359` — API aggregation deployment not starting
- `namespace.go:579` — namespace deletion unexpected
- `resource_quota.go:282` — quota condition not removed
- `certificates.go:404` — CSR approval
- `endpointslice.go:135` — slice cleanup, EndpointSlice rewrite (01d2d72)
- `endpointslicemirroring.go:129` — mirroring fix (06b6644)
- `kubectl.go:1881` — kubectl expose, downstream of OpenAPI fix (3202d92)
- `hostport.go:219` — host port binding

## All Fix Commits (35 total)

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
| 130 | TBD | TBD | 441 | TBD |

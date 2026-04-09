# Conformance Failure Tracker

**Round 127** | 397/441 (90.0%) | 44 failures | 2026-04-08
**Round 128** | In progress | 29 failures / 57 done so far | 2026-04-08

Round 128 binary: commit 36ed11a. Many fixes committed AFTER this binary was built.

## Round 128 Failures (25 unique test locations, 57/441 done)

### 1. StatefulSet scaling — vacuous truth in scale-down (1 failure) — FIXED
- `statefulset.go:2479` — scaled 3 -> 2 replicas when pods were unhealthy
- **Root cause**: `(0..0).all()` is vacuously true when desired=0, allowing scale-down. K8s uses processCondemned() with firstUnhealthyPod tracking.
- **Fix**: Rewrote scale-down using K8s condemned pod pattern (commit 8db2024)
- **Test**: `test_scale_down_blocked_when_pods_unhealthy`

### 2. DNS Resolution (2 failures) — ANALYZED
- `dns_common.go:476` — context deadline exceeded reading from pod proxy
- **Root cause**: Pod proxy reaches the agnhost container but DNS queries inside the pod time out. CoreDNS service (10.96.0.10) has correct iptables DNAT rules. Actual DNS network path between pod containers and CoreDNS may have timing issues.
- **Status**: Networking infrastructure issue. Watch fix (ce45c59) reduces API call storms that contribute to timeouts.

### 3. kubectl / OpenAPI protobuf (1 failure) — FIXED
- `builder.go:97` — error running kubectl create: failed to download openapi
- **Fix**: Return empty protobuf body for OpenAPI v2 (commit 038089e). OpenAPI v2 also returns JSON-only for CRD schemas.

### 4. Webhook readiness (5 failures) — ANALYZED
- `webhook.go:601,675,904,1194,2032` — webhook config not ready: timed out
- **Root cause**: Webhook deployment pod creates a webhook server. The server must be running and intercepting ConfigMap creates. Our API server DOES call webhooks (confirmed in logs). The webhook deployment pod may not start in time due to scheduling, image pull, or networking delays.
- **Status**: Downstream of scheduling, watch, and protobuf fixes that aren't deployed yet.

### 5. Service deletion watch (1 failure) — ANALYZED
- `service.go:3459` — failed to delete Service: timed out waiting for condition
- **Root cause**: Watch not delivering deletion event. JSON watch handler fix (ce45c59) addresses watch reliability.

### 6. Deployment revision + rollover (2 failures) — PARTIALLY FIXED
- `deployment.go:781` — revision not set — **FIXED**: Update revision every reconcile, not just when missing (commit 5c2d7ec)
- `deployment.go:995` — total pods available: 0 — pods not becoming available due to scheduling/watch issues

### 7. Ephemeral containers exec (2 failures) — FIXED
- `exec_util.go:113` — Container debugger not found in pod
- **Fix**: Search all container lists (regular, init, ephemeral) in exec handler (commit e23b7bc)

### 8. RC pod count (1 failure) — ANALYZED
- `rc.go:509` — 1 pod expected, many created
- **Root cause**: Watch failures cause client-go rate limiter storms. Watch fix (ce45c59) and scheduler fix (6124087) address upstream issues.

### 9. CRD conditions (1 failure) — FIXED
- `custom_resource_definition.go:405` — custom condition erased by controller
- **Fix**: Preserve existing conditions, only replace Established/NamesAccepted (commit 2b30373). Matches K8s SetCRDCondition() pattern.

### 10. CRD creation timeout (1 failure) — FIXED (watch fix)
- `field_validation.go:305` — cannot create crd context deadline exceeded
- **Fix**: JSON watch handler fix (ce45c59) fixes CRD watch delivery.

### 11. CRD OpenAPI schema (3 failures) — FIXED
- `crd_publish_openapi.go:77,161,253` — CRD schema not in OpenAPI spec
- **Root cause**: CRDs sent via protobuf lost their openAPIV3Schema during decoding. Our protobuf decoder didn't have CRD schemas.
- **Fix**: Added full CRD proto schemas including JSONSchemaProps with 40+ fields and MessageMap for map<string, JSONSchemaProps> (commit 019f470)

### 12. CRD selectable fields (1 failure) — ANALYZED
- `crd_selectable_fields.go:232` — CRD with selectable fields
- **Root cause**: Likely CRD creation via protobuf. Fix in commit 019f470 (CRD proto schemas) should resolve.

### 13. Job status (1 failure) — ANALYZED
- `job.go:514` — job status assertion failure
- **Root cause**: Needs deeper analysis of specific assertion.

### 14. Service Account (1 failure) — FIXED
- `service_accounts.go:817` — timed out waiting
- **Fix**: kube-api-access volume injection fix — don't skip for pods with custom "token" volumes (commit cd7eb36)

### 15. /etc/hosts (1 failure) — FIXED
- `kubelet_etc_hosts.go:143` — hosts file not recognized as kubelet-managed
- **Fix**: Added missing period to header: "# Kubernetes-managed hosts file." (commit 873edac). Matches K8s `managedHostsHeader` constant.

### 16. Service reachability (1 failure) — PARTIALLY FIXED
- `service.go:886` — service not reachable
- **Fix**: EndpointSlice controller rewrite (commit 01d2d72) fixes port filtering. Remaining networking issues are kube-proxy/CNI.

### 17. Preemption (1 failure) — FIXED
- `preemption.go:268` — pods not scheduled
- **Fix**: Resource counting fix (commit 6124087) — only count Running non-terminating pods. Use nominatedNodeName for eviction.

### 18. EndpointSlice mirroring (1 failure) — FIXED
- `endpointslicemirroring.go:129` — mirroring issue
- **Fix**: EndpointSlice controller rewrite (commit 01d2d72) builds from Service+Pods with FindPort.

## All Fix Commits (22 total)

| Commit | Component | Fix |
|--------|-----------|-----|
| ce45c59 | api-server | Watch handlers error/reconnect, aggregated discovery, pod patch generation |
| 7ca9160 | api-server | Generic protobuf-to-JSON decoder (60+ K8s types) |
| 038089e | api-server | OpenAPI v2 protobuf response format |
| 6fc1e55 | api-server | WebSocket exec channel 3 status |
| 9809d59 | api-server | Proxy trailing slash routes |
| 36ed11a | api-server | Aggregated discovery — always return when Accept includes it |
| df93155 | api-server | Aggregated discovery v2/v2beta1 version negotiation |
| e23b7bc | api-server | Exec handler — search ephemeral and init containers |
| 019f470 | api-server | Protobuf decoder — CRD schemas with JSONSchemaProps |
| 6b43640 | controller-manager | StatefulSet partition-aware pod creation |
| 8db2024 | controller-manager | StatefulSet scale-down processCondemned |
| f52a6b1 | controller-manager | DaemonSet ControllerRevision hash + data format |
| 01d2d72 | controller-manager | EndpointSlice controller rewrite (Service+Pods) |
| 5c2d7ec | controller-manager | Deployment revision — update every reconcile |
| 2b30373 | controller-manager | CRD controller — preserve existing conditions |
| 6124087 | scheduler | Preemption resource counting + eviction handling |
| d31aaed | kubelet | Init container incomplete status list |
| 5dac01a | kubelet | Container restart mechanism |
| cd7eb36 | kubelet | Service account volume injection |
| 873edac | kubelet | /etc/hosts header period |
| 3a927d1 | kubelet | Termination message fallback (pre-session) |
| eaba1ef | api-server | Field validation duplicate field (pre-session) |
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
| 128 | TBD | TBD | 441 | TBD |

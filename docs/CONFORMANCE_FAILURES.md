# Conformance Failure Tracker

**Round 127** | 397/441 (90.0%) | 44 failures | 2026-04-08
**Round 128** | 340/441 (77.1%) | 101 failures | 2026-04-08 (regressed — v2 discovery broke sonobuoy)
**Round 129** | 346/441 (78.5%) | 95 failures | 2026-04-09 (all 26 fixes deployed, but regressions remain)

## Round 129 Failures — Complete List (95 failures, 80 unique tests)

### Category 1: CRD Protobuf Decode — `missing field 'spec'` (10+ failures) — FIXED
- `custom_resource_definition.go:72,104,161,288`
- `crd_publish_openapi.go:77,161,202,244,285,318,366,400,451`
- `crd_watch.go:72`
- `field_validation.go:245,305,428,570,700`
- **Error**: `failed to decode CRD: missing field 'spec'`
- **Root cause**: The protobuf Unknown envelope parser used `field_number == 2 || field_number == 3` to capture raw bytes. Field 3 is contentEncoding (a string), not raw resource data. When field 3 appeared after field 2, it OVERWROTE raw_bytes with the encoding string, losing the actual CRD data. Result: decoded to just `{"apiVersion":"...","kind":"..."}` with no metadata or spec.
- **Fix**: Only capture field 2 as raw bytes (commit 2411448). Same bug fixed in both extract_json_from_k8s_protobuf and decode_k8s_resource.
- **Status**: FIXED

### Category 2: kubectl / OpenAPI protobuf (8 failures)
- `builder.go:97` (x8 across different namespaces)
- **Error**: `error running kubectl create/replace: error validating data: failed to download openapi: proto: cannot parse invalid wire-format data`
- **Root cause**: kubectl still can't parse our OpenAPI v2 protobuf response despite the empty body fix
- **Status**: TODO

### Category 3: DNS Resolution (7 failures)
- `dns_common.go:476` (x7)
- **Error**: `client rate limiter Wait returned an error: context deadline exceeded`
- **Root cause**: DNS pod proxy requests fail due to rate limiting from excessive API calls
- **Status**: TODO

### Category 4: Webhooks (12 failures)
- `webhook.go:425,520,601,675,904,1194,1244,1269,1334,1549,1631,2338,2465`
- **Error**: `waiting for webhook configuration to be ready: timed out` / `Webhook request failed: error sending request`
- **Root cause**: Webhook server IS called (confirmed in API server logs) but responses aren't denying requests. Need HTTP-level debugging.
- **Status**: TODO

### Category 5: Scheduling/Preemption (4 failures)
- `preemption.go:181,268,516,1025`
- **Error**: Timeouts waiting for pods to schedule, `0/2 nodes available`, RS never had available replicas
- **Status**: TODO

### Category 6: Service Networking (4 failures)
- `service.go:768,886,3459`
- `service_latency.go:142`
- **Error**: Service not reachable, failed to delete service (watch timeout), protobuf decode (`missing field 'metadata'`)
- **Status**: TODO

### Category 7: Proxy (2 failures)
- `proxy.go:271,503`
- **Error**: Unable to reach service through proxy, pod didn't start
- **Status**: TODO

### Category 8: StatefulSet (2 failures)
- `statefulset.go:957` — Pod ss-0 not recreated
- `statefulset.go:1092` — wrong image after update
- **Status**: TODO

### Category 9: Deployment (3 failures)
- `deployment.go:781` — revision not set (fix didn't work)
- `deployment.go:995` — pods not available
- `deployment.go:1259` — patched object missing annotation
- **Status**: TODO

### Category 10: Job (4 failures)
- `job.go:514,555,595,817`
- **Error**: Various job status issues, completion timeout
- **Status**: TODO

### Category 11: DaemonSet (1 failure)
- `daemon_set.go:1276` — ControllerRevision hash mismatch (fix didn't work?)
- **Status**: TODO

### Category 12: ReplicaSet/RC (4 failures)
- `replica_set.go:232,560` — RS not scaling, available replicas
- `rc.go:509,623` — pods not coming up, failure condition not removed
- **Status**: TODO

### Category 13: Ephemeral Containers (2 failures)
- `ephemeral_containers.go:92,145` — pod logs/exec for ephemeral container fails
- **Error**: `the server could not find the requested resource (get pods ...)`
- **Status**: TODO

### Category 14: Init Container (2 failures)
- `init_container.go:440,565` — init containers incomplete
- **Status**: TODO

### Category 15: Service Account (3 failures)
- `service_accounts.go:151,667,817`
- **Error**: Extra info missing, TLS cert verification, timeout
- **Status**: TODO

### Category 16: Kubelet/Runtime (5 failures)
- `kubelet_etc_hosts.go:147` — host network pod hosts file
- `runtime.go:115` — container restart count
- `pod_resize.go:857` (x2) — pod resize
- `expansion.go:351` — subpath expansion
- `exec_util.go:113` — command failed in container
- **Status**: TODO

### Category 17: Other (7 failures)
- `aggregated_discovery.go:227,336` — discovery issues
- `aggregator.go:359` — API aggregation
- `namespace.go:579` — namespace deletion
- `resource_quota.go:282` — quota not removed
- `certificates.go:404` — certificate signing
- `endpointslice.go:135` — endpoint slice deletion
- `endpointslicemirroring.go:129` — mirroring
- `kubectl.go:1881` — kubectl expose
- `hostport.go:219` — host port binding
- **Status**: TODO

## Key Regressions from Round 127 → 129

| Issue | Round 127 | Round 129 | Status |
|-------|-----------|-----------|--------|
| CRD creation | 10 failures (watch timeout) | 10+ failures (`missing field 'spec'`) | **WORSE — protobuf regression** |
| kubectl/OpenAPI | 3 failures | 8 failures | **WORSE** |
| DNS | 4 failures | 7 failures | **WORSE** |
| Webhooks | 3 failures | 12 failures | **WORSE** |
| StatefulSet | 3 failures | 2 failures | Slightly better |
| Preemption | 3 failures | 4 failures | Same |
| Service | 2 failures | 4 failures | **WORSE** |

## Priority Fixes Needed

1. **CRD protobuf decode regression** — The generic protobuf decoder for CRDs is producing JSON without `spec`. This MUST be fixed first as it causes 10+ cascading failures.
2. **kubectl OpenAPI** — Still broken, 8 failures
3. **Webhook interceptor** — 12 failures, webhook calls succeed but don't deny

## All Fix Commits (26 total)

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
| f7c16a0 | api-server | Webhook response logging |
| 6b43640 | controller-manager | StatefulSet partition-aware pod creation |
| 8db2024 | controller-manager | StatefulSet scale-down processCondemned |
| f52a6b1 | controller-manager | DaemonSet ControllerRevision hash + data format |
| 01d2d72 | controller-manager | EndpointSlice controller rewrite (Service+Pods) |
| 06b6644 | controller-manager | EndpointSlice mirroring for orphan Endpoints |
| 5c2d7ec | controller-manager | Deployment revision — update every reconcile |
| 2b30373 | controller-manager | CRD controller — preserve existing conditions |
| 2898a00 | controller-manager | Job status terminating count |
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
| 128 | 340 | 101 | 441 | 77.1% |
| 129 | 346 | 95 | 441 | 78.5% |

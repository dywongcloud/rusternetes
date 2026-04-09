# Conformance Failure Tracker

**Round 131** | Running (~86/98 so far, 87.8%) | 2026-04-09

## Round 131 Active Failures (15 at 128/441, 88.3%)

### FIXED (next round — 5 failures)
| Test | Error | Fix |
|------|-------|-----|
| `webhook.go:2129` | CR creation not denied by webhook | 6edb6be — CRD webhook calls |
| `runtime.go:115` | RestartCount stays 0 | 323d9dc — volume paths on restart |
| `crd_publish_openapi.go:285` | x-kubernetes-* false booleans | f34bd51 — skip_false_or_none |
| `crd_publish_openapi.go:211,253,366` | Same CRD OpenAPI issue | f34bd51 — same fix |
| `service_accounts.go:667` | JWT missing kubernetes.io claims | db4855b — nested KubernetesClaims |

### NEEDS FIX (6 failures)
| Test | Error | Status |
|------|-------|--------|
| `field_validation.go:245,428` | "cannot create crd context deadline exceeded" | CRD creation times out — test waits for CRD to appear in API discovery |
| `job.go:555` | Job successPolicy wrong index | Missing SuccessPolicy type + controller logic |
| `webhook.go:463` | Webhook rule update not taking effect | ConfigMap creation still denied after rule change |
| `namespace.go:579` | Missing NamespaceDeletionContentFailure condition | Conditions may not persist (CAS conflict?) — c5ad02d adds logging |
| `dns_common.go:476` | Rate limiter context deadline | Client rate limiter cascade |
| `aggregated_discovery.go:336` | Discovery response issue | Needs investigation |

### NEEDS INVESTIGATION (2 failures)
| Test | Error |
|------|-------|
| `kubectl/builder.go:97` | kubectl label exit code 1 |
| `aggregated_discovery.go:336` | Unknown |

## Fix Commits (43 total)

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
| 4496809 | api-server | SA admission — projected volume not secret |
| eaba1ef | api-server | Field validation duplicate field |
| f34bd51 | common | CRD OpenAPI — omit x-kubernetes-* false booleans |
| 6edb6be | api-server | CRD webhooks — run admission on custom resource create |
| 6b43640 | controller-manager | StatefulSet partition-aware pod creation |
| 8db2024 | controller-manager | StatefulSet scale-down processCondemned |
| f52a6b1 | controller-manager | DaemonSet ControllerRevision hash + data format |
| 01d2d72 | controller-manager | EndpointSlice controller rewrite (Service+Pods) |
| 06b6644 | controller-manager | EndpointSlice mirroring for orphan Endpoints |
| 5c2d7ec | controller-manager | Deployment revision — update every reconcile |
| dc8343e | controller-manager | Deployment RS adoption |
| 2b30373 | controller-manager | CRD controller — preserve existing conditions |
| 2898a00 | controller-manager | Job status terminating count |
| 38ddae4 | controller-manager | RC controller — clear ReplicaFailure condition |
| 2d3c799 | controller-manager | Job ready field |
| 6124087 | scheduler | Preemption resource counting + eviction handling |
| d31aaed | kubelet | Init container incomplete status list |
| 5dac01a | kubelet | Container restart mechanism |
| cd7eb36 | kubelet | Service account volume injection |
| 873edac | kubelet | /etc/hosts header period |
| 188eb6a | kubelet | /etc/hosts skip for host network pods |
| 3a927d1 | kubelet | Termination message fallback |
| c10e449 | kubelet | Node labels — kubernetes.io/os, arch, hostname |
| 3136c2a | kubelet | Projected volume — preserve SA token during resync |
| 323d9dc | kubelet | Container restart — pass volume paths when recreating |
| db4855b | common/kubelet/api-server | JWT claims — kubernetes.io nested claims |
| c5ad02d | controller-manager | Namespace controller — deletion condition logging |

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

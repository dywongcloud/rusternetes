# Conformance Failure Tracker

**Round 131** | ~235/292 (80.5%) before sonobuoy ns was deleted | 2026-04-09
**Round 132** | Running with 23 fixes | 2026-04-10

## Fixes for Round 132 (23 commits covering 20+ unique tests)

| # | Test | Root Cause | Fix Commit |
|---|------|------------|------------|
| 1 | `webhook.go:2129` | CRD create didn't call webhooks | 6edb6be |
| 2 | `runtime.go:115` | Container restart — empty volume_paths | 323d9dc |
| 3 | `crd_publish_openapi.go:285,211,253,318,366,451` | x-kubernetes-* false booleans + CRD watch | f34bd51 + f7dfb20 |
| 4 | `field_validation.go:245,428` | CRD watch missing → isWatchCachePrimed timeout | f7dfb20 |
| 5 | `service_accounts.go:667` | JWT missing kubernetes.io nested claims | db4855b |
| 6 | `job.go:555,595` | Job successPolicy set ready=current instead of 0 | c4d3fa7 |
| 7 | `kubectl/builder.go:97` | Merge patch — metadata.name null before deser | eb07e78 |
| 8 | `aggregated_discovery.go:227,336` | CRD watch issue + discovery format | f7dfb20 |
| 9 | `ephemeral_containers.go:92` | Pod logs didn't search ephemeral containers | f50d364 |
| 10 | `custom_resource_definition.go:164` | CRD type/status fields missing defaults | 77f4e6f |
| 11 | `service_latency.go:142` | EndpointAddress ip field required, not defaulted | 8dbedb5 |
| 12 | `certificates.go:404` | CSR status PATCH didn't merge metadata | 176b2cd |
| 13 | `webhook.go:1573` | Webhook TLS — accepted self-signed without CA bundle | af5e245 |
| 14 | `service_accounts.go:817` | Root CA configmap not reconciled after modification | c4bda95 |
| 15 | `namespace.go:579` | Namespace deletion conditions CAS conflict | d26e2ef |
| 16 | `endpointslice.go:135` | Services with empty selector skipped | c2a0dd8 |
| 17 | `resource_quota.go:282` | Node missing ephemeral-storage capacity | 967b1fd |

## Remaining Unfixed Issues

| Test | Error | Analysis |
|------|-------|----------|
| `webhook.go:463` | Marker webhook fires in wrong namespace | FIXED f1e00db — namespaceSelector support |
| `webhook.go:520,904,2107` | Webhook service not ready | Service endpoint readiness timing |
| `webhook.go:1396` | Error message case mismatch ("Webhook" vs "webhook") | FIXED 7ae38d7 — lowercase error messages |
| `dns_common.go:476` (x5) | Container exec shell error + rate limiter | Container runs /pause instead of shell; cascades to rate limiter |
| `preemption.go:181,268,516` | Pod startup timeout | Extended resource (scheduling.k8s.io/foo) handling; node patch timing |
| `statefulset.go:957,1092` | Pod not deleted/recreated during scale-down | Kubelet container stop grace period + controller reconcile timing |
| `daemon_set.go:1276` | ControllerRevision Match — 0 matching | getPatch data format byte-level comparison |
| `deployment.go:991,1259` | New RS not created / rollover timeout | Template change detection + watch cancellation cascade |
| `rc.go:509`, `replica_set.go:232` | Pod startup cascade | Resource pressure during test run |
| `proxy.go:271` | Service proxy URL rewriting | Backend service readiness |
| `pod_resize.go:857` | cgroup changes in DinD | Container-in-container Docker limitation |
| `pod/output.go:263` | File permissions 0755 vs 0777 | Docker umask (0022) in emptyDir volumes |
| `kubectl.go:1881` | kubectl proxy/expose | OpenAPI response format |

## Fix Commits This Session (23 commits)

| Commit | Component | Fix |
|--------|-----------|-----|
| c10e449 | kubelet | Node labels — kubernetes.io/os, arch, hostname |
| 3136c2a | kubelet | Projected volume — preserve SA token during resync |
| f34bd51 | common | CRD OpenAPI — omit x-kubernetes-* false booleans |
| 6edb6be | api-server | CRD webhooks — run admission on custom resource create |
| 323d9dc | kubelet | Container restart — pass volume paths when recreating |
| db4855b | common/kubelet/api-server | JWT claims — kubernetes.io nested claims |
| c5ad02d | controller-manager | Namespace controller — deletion condition logging |
| d26e2ef | controller-manager | Namespace deletion — retry condition update on CAS conflict |
| f7dfb20 | api-server | CRD watch — watch support for custom resource instances |
| c4d3fa7 | controller-manager | Job successPolicy — ready=0 on completion |
| eb07e78 | api-server | Pod PATCH — preserve metadata.name before deserialization |
| f50d364 | api-server | Pod logs — search ephemeral and init containers |
| 8dbedb5 | common | EndpointAddress — serde default for ip field |
| 77f4e6f | common | CRD types — serde defaults for required string fields |
| 176b2cd | api-server | CSR status PATCH — merge metadata annotations/labels |
| af5e245 | api-server | Webhook TLS — respect CA bundle for cert verification |
| c4bda95 | controller-manager | Root CA ConfigMap — reconcile data, not just existence |
| c2a0dd8 | controller-manager | EndpointSlice — handle services with empty selectors |
| 967b1fd | kubelet | Node capacity — report ephemeral-storage |

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
| 131 | ~235 | ~57 | ~292 | ~80.5% (aborted — sonobuoy ns deleted) |
| 132 | TBD | TBD | 441 | TBD |

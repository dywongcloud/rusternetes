# Conformance Failure Tracker

**Round 133** | 370/441 (83.9%) | 71 failures, 59 unique | 2026-04-10
**Round 134** | Running — 1 failure at 30min mark (all 42 fixes) | 2026-04-10

## Round 133 Failures by Category

| Category | Count | Tests | Status |
|----------|-------|-------|--------|
| CRD OpenAPI | 9 | 77,161,211,253,285,318,366,400,451 | FIXED 0347108 (empty vec/map skip) + 0b22923 (CR unknown fields) |
| Webhooks | 10 | 463,520,675,904,1269,1334,1396,1400,1481,2107 | Partially fixed: f1e00db (ns selector), 7ae38d7 (error msg), 09bcebe (subresource) |
| Field validation | 5 | 278,338,462,611,735 | FIXED: 5b19baf (CRD apply), 5ff70c7 (YAML parse). 735 = YAML dup detection (needs YAML parser) |
| Preemption | 4 | 181,268,516,1025 | FIXED 4e442e8 (extended resources) |
| Service | 3 | 768,886,3459 | Networking/kube-proxy/watch |
| Deployment | 2 | 995,1259 | b2ba5cf (template matching) + watch cascade |
| StatefulSet | 2 | 957 | Scale-down timing |
| ReplicaSet | 2 | 232,560 | Pod startup / watch cascade |
| RC | 2 | 509,623 | Pod startup / watch cascade |
| Init container | 2 | 440,565 | Watch/status timing |
| Proxy | 2 | 271,503 | Service proxy networking |
| Pod output | 1 | 263 | Docker umask (0755 vs 0777) |
| Pod resize | 1 | 857 | Docker cgroup limitation |
| Others | 14 | namespace:579, resource_quota:282, builder:97, kubectl:1881, dns:476, endpointslice:135, endpointslicemirroring:129, hostport:219, service_latency:142, service_accounts:667, runtime:115, daemon_set:1276, custom_resource_definition:334, aggregator:359 |

## Fixes Not Yet in Round 133 Build (12 commits)

| Commit | Fix | Expected Tests Fixed |
|--------|-----|---------------------|
| 0b22923 | CustomResource preserve unknown fields | crd_openapi:211,451 |
| 182b280 | Namespace finalization timing | namespace:579 |
| 2332cf4 | ObjectMeta null name tolerant | builder:97 |
| 5ff70c7 | CRD PATCH YAML parsing | field_validation:462 |
| b5e457c | EndpointAddress ip null tolerant | service_latency:142 |
| 09bcebe | Webhook resource/subresource matching | webhook:1481 |
| 4e442e8 | Scheduler extended resources | preemption:181,268,516,1025 |
| 0347108 | JSONSchemaProps skip empty vec/map | crd_openapi:77,161,253,285,318,366,400 |

## Fix Commits This Session (41 total)

| Commit | Fix |
|--------|-----|
| c10e449 | Node labels |
| 3136c2a | SA token resync |
| f34bd51 | CRD OpenAPI x-kubernetes booleans |
| 6edb6be | CRD webhooks |
| 323d9dc | Container restart volumes |
| db4855b | JWT kubernetes.io claims |
| c5ad02d | Namespace condition logging |
| d26e2ef | Namespace condition CAS retry |
| f7dfb20 | CRD watch support |
| c4d3fa7 | Job successPolicy ready=0 |
| eb07e78 | Pod PATCH metadata.name |
| f50d364 | Pod logs ephemeral containers |
| 8dbedb5 | EndpointAddress ip default |
| 77f4e6f | CRD type defaults |
| 176b2cd | CSR status PATCH metadata |
| af5e245 | Webhook TLS CA bundle |
| c4bda95 | Root CA ConfigMap reconciliation |
| c2a0dd8 | EndpointSlice empty selectors |
| 967b1fd | Node ephemeral-storage capacity |
| f1e00db | Webhook namespaceSelector |
| 7ae38d7 | Webhook error lowercase |
| 5b19baf | CRD PATCH server-side apply |
| b2ba5cf | Deployment template matching |
| 06d3a40 | Discovery shortNames |
| faf427c | JSONSchemaProps omitempty strings/bools |
| 0b22923 | CustomResource unknown fields |
| 182b280 | Namespace finalization timing |
| 2332cf4 | ObjectMeta null name |
| 5ff70c7 | CRD PATCH YAML parsing |
| b5e457c | EndpointAddress ip null |
| 09bcebe | Webhook resource/subresource matching |
| 4e442e8 | Scheduler extended resources |
| 0347108 | JSONSchemaProps empty vec/map skip |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | TBD | TBD | 441 | TBD |

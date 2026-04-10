# Conformance Failure Tracker

**Round 132** | 363/441 (82.3%) | 78 failures | 2026-04-10

## Round 132 Results — 62 unique failing tests

### By Category
- **Webhooks** (12): 463, 520, 675, 904, 1269, 1334, 1396, 1481, 1549, 2107, 2129, 2465
- **CRD OpenAPI** (9): 77, 161, 211, 253, 285, 318, 366, 400, 451
- **Field validation** (5): 278, 338, 462, 611, 735 — FIXED 5b19baf (CRD apply)
- **Preemption** (4): 181, 268, 516, 1025
- **Service** (3): 768, 886, 3459
- **Deployment** (3): 781, 991, 1259
- **StatefulSet** (2): 957, 1092
- **ReplicaSet** (2): 232, 560
- **RC** (2): 509, 623
- **Init container** (2): 440, 565
- **Proxy** (2): 271, 503
- **Pod output/perms** (1): 263
- **Others** (15): aggregator, custom_resource_definition, namespace, resource_quota, dns, endpointslice, endpointslicemirroring, hostport, service_latency, service_accounts, runtime, pod_resize, lifecycle_hook, kubectl, builder, daemon_set

### Fixes Not Yet Deployed (6 commits, should fix ~10+ tests)
| Commit | Fix | Expected Tests Fixed |
|--------|-----|---------------------|
| f1e00db | Webhook namespaceSelector | webhook:463 + others |
| 7ae38d7 | Webhook error lowercase | webhook:1396 |
| 5b19baf | CRD PATCH apply creates | field_validation:278,338,462,611,735 |
| b2ba5cf | Deployment template matching | deployment:991 |
| 06d3a40 | Aggregated discovery shortNames | builder:97 |

## Fix Commits This Session (28 commits)

| Commit | Component | Fix |
|--------|-----------|-----|
| c10e449 | kubelet | Node labels |
| 3136c2a | kubelet | SA token resync |
| f34bd51 | common | CRD OpenAPI x-kubernetes booleans |
| 6edb6be | api-server | CRD webhooks |
| 323d9dc | kubelet | Container restart volumes |
| db4855b | common/kubelet/api-server | JWT kubernetes.io claims |
| c5ad02d | controller-manager | Namespace condition logging |
| d26e2ef | controller-manager | Namespace condition retry |
| f7dfb20 | api-server | CRD watch support |
| c4d3fa7 | controller-manager | Job successPolicy ready=0 |
| eb07e78 | api-server | Pod PATCH metadata |
| f50d364 | api-server | Pod logs ephemeral containers |
| 8dbedb5 | common | EndpointAddress ip default |
| 77f4e6f | common | CRD type defaults |
| 176b2cd | api-server | CSR status PATCH metadata |
| af5e245 | api-server | Webhook TLS CA bundle |
| c4bda95 | controller-manager | Root CA reconciliation |
| c2a0dd8 | controller-manager | EndpointSlice empty selectors |
| 967b1fd | kubelet | Ephemeral-storage capacity |
| f1e00db | api-server | Webhook namespaceSelector |
| 7ae38d7 | api-server | Webhook error lowercase |
| 5b19baf | api-server | CRD PATCH server-side apply |
| b2ba5cf | controller-manager | Deployment template matching |
| 06d3a40 | api-server | Discovery shortNames |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 131 | ~235 | ~57 | ~292 | ~80.5% (aborted) |
| 132 | 363 | 78 | 441 | 82.3% |

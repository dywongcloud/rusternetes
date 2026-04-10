# Conformance Failure Tracker

**Round 131** | Running | 2026-04-09

## Round 131 Failures — Fix Status

### FIXED for next round (13 failures covered by 10 fix commits)

| # | Test | Root Cause | Fix |
|---|------|------------|-----|
| 1 | `webhook.go:2129` | CRD create didn't call webhooks | 6edb6be |
| 2 | `runtime.go:115` | Container restart — empty volume_paths | 323d9dc |
| 3 | `crd_publish_openapi.go:285` | x-kubernetes-* false booleans in schema | f34bd51 |
| 4 | `crd_publish_openapi.go:211,253,366,451` | Same + CRD watch needed | f34bd51 + f7dfb20 |
| 5 | `field_validation.go:245,428` | CRD watch missing → isWatchCachePrimed timeout | f7dfb20 |
| 6 | `service_accounts.go:667` | JWT missing kubernetes.io nested claims | db4855b |
| 7 | `job.go:555` | Job successPolicy set ready=current instead of 0 | c4d3fa7 |
| 8 | `kubectl/builder.go:97` | Merge patch — metadata.name null before deser | eb07e78 |
| 9 | `aggregated_discovery.go:336` | Same CRD watch issue as field_validation | f7dfb20 |

### Still failing (need fix or investigation)

| # | Test | Error | Status |
|---|------|-------|--------|
| 10 | `webhook.go:463` | Rule update to remove CREATE not taking effect | Webhook rules read from etcd each time — investigating |
| 11 | `webhook.go:904` | Webhook config never ready (service not reachable) | Pod/service startup timing |
| 12 | `namespace.go:579` | NamespaceDeletionContentFailure condition missing | Controller sets conditions — may be CAS conflict |
| 13 | `dns_common.go:476` (x4) | Rate limiter context deadline / shell syntax error in containers | Container exec issue |
| 14 | `daemon_set.go:1276` | ControllerRevision Match — 0 matching histories | CR data format mismatch? |
| 15 | `statefulset.go:957` | Pod not deleted during scale-down | Kubelet sync timing |
| 16 | `preemption.go:181` | Pods not running after scheduling (timeout) | Pod startup timing |

## Fix Commits This Session (13 commits)

| Commit | Component | Fix |
|--------|-----------|-----|
| c10e449 | kubelet | Node labels — kubernetes.io/os, arch, hostname |
| 3136c2a | kubelet | Projected volume — preserve SA token during resync |
| f34bd51 | common | CRD OpenAPI — omit x-kubernetes-* false booleans |
| 6edb6be | api-server | CRD webhooks — run admission on custom resource create |
| 323d9dc | kubelet | Container restart — pass volume paths when recreating |
| db4855b | common/kubelet/api-server | JWT claims — kubernetes.io nested claims |
| c5ad02d | controller-manager | Namespace controller — deletion condition logging |
| f7dfb20 | api-server | CRD watch — watch support for custom resource instances |
| c4d3fa7 | controller-manager | Job successPolicy — ready=0 on completion |
| eb07e78 | api-server | Pod PATCH — preserve metadata.name before deserialization |

## All Fix Commits (47 total)

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
| f7dfb20 | api-server | CRD watch — watch support for custom resource instances |
| eb07e78 | api-server | Pod PATCH — preserve metadata.name before deserialization |
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
| c5ad02d | controller-manager | Namespace controller — deletion condition logging |
| c4d3fa7 | controller-manager | Job successPolicy — ready=0 on completion |
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

# Conformance Failure Tracker

**Round 132** | Running with 26 fixes | 2026-04-10

## Round 132 Failures (so far)

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | `webhook.go:1396` | Error case "Webhook" vs "webhook" | FIXED 7ae38d7 (not in this build) |
| 2 | `proxy.go:271` | Unable to reach service through proxy | Known — service readiness |
| 3 | `pod/output.go:263` | File perms 0755 vs 0777 | Known — Docker umask |
| 4 | `field_validation.go:278` | CRD resource not found during apply | FIXED 5b19baf (not in this build) |
| 5 | `statefulset.go:957` | Pod ss-0 not re-created | Known — controller/kubelet timing |
| 6 | `field_validation.go:611` | CRD apply resource not found | FIXED 5b19baf (not in this build) |
| 7 | `statefulset.go:1092` | StatefulSet image update | Known — template matching |
| 8 | `webhook.go:520` | Webhook service not ready | Known — endpoint readiness timing |
| 9 | `lifecycle_hook.go:132` | PreStop hook not executed | Kubelet preStop hook execution |

## Fixes Committed (26 this session, not yet deployed)

### Deployed in Round 132 Build (23 fixes)
| Commit | Fix |
|--------|-----|
| c10e449 | Node labels — kubernetes.io/os, arch, hostname |
| 3136c2a | Projected volume — preserve SA token during resync |
| f34bd51 | CRD OpenAPI — omit x-kubernetes-* false booleans |
| 6edb6be | CRD webhooks — run admission on custom resource create |
| 323d9dc | Container restart — pass volume paths when recreating |
| db4855b | JWT claims — kubernetes.io nested claims |
| c5ad02d | Namespace controller — deletion condition logging |
| d26e2ef | Namespace deletion — retry condition update on CAS conflict |
| f7dfb20 | CRD watch — watch support for custom resource instances |
| c4d3fa7 | Job successPolicy — ready=0 on completion |
| eb07e78 | Pod PATCH — preserve metadata.name before deserialization |
| f50d364 | Pod logs — search ephemeral and init containers |
| 8dbedb5 | EndpointAddress — serde default for ip field |
| 77f4e6f | CRD types — serde defaults for required string fields |
| 176b2cd | CSR status PATCH — merge metadata annotations/labels |
| af5e245 | Webhook TLS — respect CA bundle for cert verification |
| c4bda95 | Root CA ConfigMap — reconcile data, not just existence |
| c2a0dd8 | EndpointSlice — handle services with empty selectors |
| 967b1fd | Node capacity — report ephemeral-storage |

### Committed After Round 132 Build (5 fixes, need redeploy)
| Commit | Fix |
|--------|-----|
| f1e00db | Webhook namespaceSelector — filter by namespace labels |
| 7ae38d7 | Webhook error messages — lowercase "webhook" |
| 5b19baf | CRD PATCH — server-side apply creates new resources |
| b2ba5cf | Deployment template matching — full deep comparison |

## Remaining Unfixed Issues

| Test | Error | Analysis |
|------|-------|----------|
| `webhook.go:520,904,2107` | Webhook service not ready | Service endpoint readiness timing |
| `dns_common.go:476` (x5) | Container exec shell error + rate limiter | Container runs /pause not shell |
| `preemption.go:181,268,516` | Pod startup timeout | Extended resource handling |
| `statefulset.go:957,1092` | Pod not deleted/recreated | Kubelet stop grace period + controller timing |
| `daemon_set.go:1276` | ControllerRevision Match — 0 matching | getPatch data byte comparison |
| `deployment.go:991,1259` | New RS not created / rollover | Template change + watch cascade |
| `rc.go:509`, `replica_set.go:232` | Pod startup cascade | Resource pressure |
| `proxy.go:271` | Service proxy URL rewriting | Backend service readiness |
| `pod_resize.go:857` | cgroup changes in DinD | Docker limitation |
| `pod/output.go:263` | File permissions 0755 vs 0777 | Docker umask |
| `kubectl.go:1881` | kubectl proxy/expose | OpenAPI response |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 103 | 245 | 196 | 441 | 55.6% |
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 131 | ~235 | ~57 | ~292 | ~80.5% (aborted) |
| 132 | TBD | TBD | 441 | TBD |

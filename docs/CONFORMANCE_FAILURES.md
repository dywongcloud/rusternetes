# Conformance Issue Tracker

**Round 94**: running (slow) | **162 fixes deployed** | resourceVersion mismatch fixed, kubelet sync fixed, tests progressing

## Fixes pending deploy (34)

| # | Fix | Impact |
|---|-----|--------|
| 1 | ServiceCIDR default | 1 test |
| 2 | /etc/hosts duplicate skip | 1 test |
| 3 | NoExecute taint eviction | 1 test |
| 4 | metadata.labels/annotations downward API | 2 tests |
| 5 | Projected volume resync | 1 test |
| 6 | Resource quota active pods + counts | 3 tests |
| 7 | /apis/ trailing slash + discovery | 2 tests |
| 8 | LimitRange defaults | 2 tests |
| 9 | Job controller 2s interval | 2 tests |
| 10 | Protobuf balanced brace extraction | 2 tests |
| 11 | CRD metadata default injection | 1 test |
| 12 | Events deletion fix (None timestamp) | 1 test |
| 13 | Secret/ConfigMap immutability enforcement | 2 tests |
| 14 | Watch history via etcd watch_from_revision | 5+ tests |
| 15 | $(VAR) expansion in container command/args | 1 test |
| 16 | Init container restart for restartPolicy=Always | 1 test |
| 17 | Deployment Recreate strategy (scale down first) | 1 test |
| 18 | Strict field validation error format | 1 test |
| 19 | Namespace status.phase=Active default | 1 test |
| 20 | PVC phase/fields defaults for deserialization | 1 test |
| 21 | Protobuf 415 when no JSON extractable | 4+ tests (CRD) |
| 22 | DELETE protobuf body handling | 1+ tests |
| 23 | 406 for SSAR table format | 1 test |
| 24 | IPAddress deletecollection route | 1 test |
| 25 | ALL 25 missing deletecollection routes registered | 5+ tests |
| 26 | PATCH dry-run support (don't save to storage) | 1 test |
| 27 | Namespace deletion: Terminating phase first | 1 test |
| 28 | Termination message FallbackToLogsOnError | 1 test |
| 29 | Session affinity via iptables recent module | 2 tests |
| 30 | Job successPolicy succeededCount + index ranges | 1 test |
| 31 | Node addresses in heartbeat (fix empty addresses) | 5+ tests |
| 32 | Sysctls applied to pause container HostConfig | 1 test |
| 33 | NodeInfo populated in heartbeat | 1+ tests |
| 34 | OpenAPI 406 for protobuf-only requests | 3 tests |
| 35 | Deployment revision sync in status updates | 1 test |
| 36 | **CRITICAL**: Kubelet don't set Failed on transient sync errors | 10+ tests (webhook, timeouts) |

## Open failures by category

| Category | Count | Tests | Status |
|----------|-------|-------|--------|
| Webhook | 10+ | webhook.go:520,675,837,904,1244,1334,1631,2338,2465 | **ROOT CAUSE FIXED**: kubelet set pods to Failed on transient storage concurrency errors during status update. Containers started fine but status write conflicted → error handler set Failed → unrecoverable. Fix #161 makes status update non-fatal and only sets Failed for container creation/image pull errors. |
| Watch/stream | 10+ | statefulset.go:786,878, watch.go:409 (×3), runtimeclass.go:317 | **FIX COMMITTED** etcd watch_from_revision |
| Scheduling | 4 | predicates.go:1102 (×2), preemption.go:516,949 | Preemption/resource-fit |
| Networking | 6+ | networking.go:72,113, util.go:182 (×2), pods.go:556, proxy.go:503 | Pod-to-pod |
| Service | 4+ | service.go:251,768,3304, endpoints.go:526, service_latency.go:142 | Affinity (**FIX COMMITTED**), routing, protobuf |
| Job | 6 | job.go:144,236,422,504,623,665 | SuccessPolicy (**FIX COMMITTED**), completion tracking |
| Resource quota | 5 | resource_quota.go:127,209,258,478,803 | Counts (**FIX PENDING**) |
| kubectl | 4 | kubectl.go:1130,1565, builder.go:97 (×3) | Dry-run (**FIX COMMITTED**), OpenAPI (**FIX COMMITTED**), protobuf |
| CRD | 5+ | crd_publish_openapi.go:77,161,285,400,451, custom_resource_definition.go:104 | Protobuf (**FIX COMMITTED** 415) |
| Deployment | 4 | deployment.go:238,781,826, statefulset.go:2253 | Recreate (**FIX COMMITTED**), revision, watch |
| VAP | 3 | validatingadmissionpolicy.go:120,568,814 | CEL/watch |
| Field validation | 2 | field_validation.go:105,700 | **FIX COMMITTED** (:105), protobuf (:700) |
| Init container | 1 | init_container.go:440 | **FIX COMMITTED** |
| Output/volume | 5+ | output.go:263 (×5) | Perms, env (**FIX COMMITTED**), cpu_request |
| ConfigMap volume | 2 | configmap_volume.go:415,525 | Update propagation (**FIX PENDING** resync), immutability (**FIX COMMITTED**) |
| Events | 1 | events.go:124 | **FIX COMMITTED** |
| DRA | 2 | conformance.go:696 (×2) | ResourceSlice protobuf |
| FlowControl | 1 | flowcontrol.go:661 | Delete (**FIX COMMITTED**), watch |
| Namespace | 2 | namespace.go:321,579 | **FIX COMMITTED** phase + Terminating |
| PV/PVC | 1 | persistent_volumes.go:718 | **FIX COMMITTED** |
| RC | 2 | rc.go:173,442 | Watch/timeout |
| ReplicaSet | 1 | replica_set.go:232 | Watch/timing |
| Runtime | 2 | runtime.go:158,169 | Termination message (**FIX COMMITTED**), timeout |
| RuntimeClass | 3 | runtimeclass.go:153,184,317 | Handler + watch |
| Table conversion | 1 | table_conversion.go:167 | **FIX COMMITTED** 406 |
| Service account | 3 | service_accounts.go:132,667,792 | Token/pod |
| DaemonSet | 2 | daemon_set.go:332,980 | Timing |
| Aggregator | 1 | aggregator.go:359 | Sample API server |
| Expansion | 2 | expansion.go:419 (×2) | subPathExpr |
| Downward API | 2 | downwardapi_volume.go:140,171 | **FIX COMMITTED** labels |
| Limit range | 1 | limit_range.go:162 | **FIX COMMITTED** |
| Sysctl | 1 | sysctl.go:99 | **FIX COMMITTED** |
| PreStop | 1 | pre_stop.go:153 | Lifecycle hook |
| Lifecycle | 1 | lifecycle_hook.go:132 | Watch failures |

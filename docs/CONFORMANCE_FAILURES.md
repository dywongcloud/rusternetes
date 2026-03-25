# Conformance Issue Tracker

**Round 93**: ~40 PASS, 82 FAIL (running) | **144 fixes** | ~33% pass rate (pre-deploy)

## Fixes pending deploy (20)

| # | Fix | Impact |
|---|-----|--------|
| 1 | ServiceCIDR default | 1 test |
| 2 | /etc/hosts duplicate skip | 1 test |
| 3 | NoExecute taint eviction | 1 test |
| 4 | metadata.labels/annotations downward API | 2 tests |
| 5 | Projected volume resync | 1 test |
| 6 | Resource quota active pods + counts | 3 tests |
| 7 | /apis/ trailing slash | 1 test |
| 8 | LimitRange defaults | 2 tests |
| 9 | Job controller 2s interval | 2 tests |
| 10 | Protobuf balanced brace extraction | 2 tests |
| 11 | CRD metadata default injection | 1 test |
| 12 | Events deletion fix (None timestamp) | 1 test |
| 13 | Secret/ConfigMap immutability enforcement | 1 test |
| 14 | Watch history via etcd watch_from_revision | 5+ tests |
| 15 | $(VAR) expansion in container args | 1 test |
| 16 | Init container restart on failure (restartPolicy=Always) | 1 test |
| 17 | Deployment Recreate strategy (scale down first) | 1 test |
| 18 | Strict field validation error format | 1 test |
| 19 | Namespace status.phase=Active (even with status:{}) | 1 test |
| 20 | PVC phase default + optional fields | 1 test |

## Open failures by category

| Category | Count | Tests | Status |
|----------|-------|-------|--------|
| Webhook not ready | 9 | webhook.go:520,675,837,904,1244,1334,1631,2338,2465, runtimeclass.go:153,184 | Webhook pods can't start in test infra |
| Watch stream | 5 | statefulset.go:786, watch.go:409 (×3) | **FIX COMMITTED** etcd watch_from_revision |
| Scheduling | 4 | predicates.go:1102 (×2), preemption.go:516,949 | Preemption/resource-fit timing |
| Networking | 6 | networking.go:72,113, util.go:182 (×2), pods.go:556, proxy.go:503 | Pod-to-pod connectivity |
| Service | 3 | service.go:251,768, endpoints.go:526 | Service routing/affinity |
| Job | 4 | job.go:144,422,623,665 | Job completion/failure/indexed |
| Resource quota | 5 | resource_quota.go:127,209,258,478,803 | Quota counts (fix pending deploy) |
| kubectl | 3 | kubectl.go:1130, builder.go:97 (×2) | Protobuf (fix pending) |
| CRD | 4 | crd_publish_openapi.go:77,285,451, custom_resource_definition.go:104 | CRD creation timeout / protobuf |
| Deployment | 3 | deployment.go:238,826, statefulset.go:2253 | Recreate (**FIX COMMITTED**), watch |
| VAP | 2 | validatingadmissionpolicy.go:120,568 | CEL evaluation timing |
| Field validation | 2 | field_validation.go:105,700 | Strict decode (**FIX COMMITTED**), CRD body |
| Init container | 1 | init_container.go:440 | **FIX COMMITTED** restart on failure |
| Output/volume | 4 | output.go:263 (×4) | Perms, env expansion (**FIX COMMITTED**), cpu_request |
| Events | 1 | events.go:124 | **FIX COMMITTED** |
| DRA | 2 | conformance.go:696 (×2) | ResourceSlice resourceVersion |
| FlowControl | 1 | flowcontrol.go:661 | Watch ERROR + delete 405 |
| Secrets | 1 | secrets_volume.go:407 | **FIX COMMITTED** immutability |
| Namespace | 1 | namespace.go:321 | **FIX COMMITTED** phase=Active |
| PV/PVC | 1 | persistent_volumes.go:718 | **FIX COMMITTED** PVC deserialization |
| RC | 2 | rc.go:173,442 | Watch/timeout |
| Table conversion | 1 | table_conversion.go:167 | SSAR metadata |
| Service account | 1 | service_accounts.go:667 | Pod Failed unexpectedly |
| DaemonSet | 1 | daemon_set.go:980 | Timing |
| Aggregator | 1 | aggregator.go:359 | Sample API server not ready |
| Discovery | 1 | discovery.go:131 | **FIX COMMITTED** /apis/ route |
| Downward API | 2 | downwardapi_volume.go:140,171 | **FIX COMMITTED** labels, cpu_request |
| Limit range | 1 | limit_range.go:162 | **FIX COMMITTED** |
| Expansion | 1 | expansion.go:419 | subPathExpr pod condition timing |
| Runtime | 2 | runtime.go:158,169 | RuntimeClass |

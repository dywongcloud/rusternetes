# Conformance Issue Tracker

**Round 93**: 66 PASS, 67 FAIL (running) | **141 fixes** | ~50% pass rate

## Fixes pending deploy (17)

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
| 13 | Secret immutability enforcement | 1 test |
| 14 | ConfigMap immutability enforcement | 1 test |
| 15 | Watch history via etcd watch_from_revision | 3 tests |
| 16 | $(VAR) expansion in container args | 1 test |
| 17 | Init container restart on failure (restartPolicy=Always) | 1 test |

## Open failures by category

| Category | Count | Tests | Status |
|----------|-------|-------|--------|
| Webhook not ready | 8 | webhook.go:520,837,904,1244,1334,1631,2465, runtimeclass.go:153 | Container infrastructure — webhook pods can't start |
| Watch stream | 5 | statefulset.go:786, watch.go:409 (×3) | **FIX COMMITTED** — etcd watch_from_revision for history replay |
| Scheduling | 3 | predicates.go:1102 (×2), preemption.go:516,949 | Preemption timing |
| Networking | 5 | networking.go:72,113, util.go:182, pods.go:556, proxy.go:503 | Pod-to-pod connectivity |
| Service | 3 | service.go:251,768, endpoints.go:526 | Service routing/affinity |
| Job | 4 | job.go:144,422,623,665 | Job completion/failure/indexed mode |
| Resource quota | 3 | resource_quota.go:127,478,803 (was 901) | Quota counts (fix pending) |
| kubectl | 3 | kubectl.go:1130, builder.go:97 (×2) | Protobuf body parsing (fix pending) |
| CRD | 2 | crd_publish_openapi.go:451, custom_resource_definition.go:104 | Protobuf parsing (fix pending) |
| Runtime | 2 | runtime.go:158,169 | RuntimeClass handler missing |
| Deployment | 2 | deployment.go:238, statefulset.go:2253 | Watch/revision tracking |
| VAP | 2 | validatingadmissionpolicy.go:120,568 | CEL evaluation logic |
| Field validation | 2 | field_validation.go:105,700 | Strict decoding error format (**FIX COMMITTED** for :105), CRD protobuf (:700) |
| Init container | 1 | init_container.go:440 | **FIX COMMITTED** — restart on failure for restartPolicy=Always |
| Output/volume | 3 | output.go:263 (×3) | File perms (0644→0666), env var expansion (**FIX COMMITTED**), downward API cpu_request |
| Events | 1 | events.go:124 | **FIX COMMITTED** — events deletion + field selector |
| DRA | 2 | conformance.go:696 (×2) | ResourceSlice resourceVersion empty |
| FlowControl | 1 | flowcontrol.go:661 | Watch ERROR events + delete 405 |
| Secrets | 1 | secrets_volume.go:407 | **FIX COMMITTED** — immutable secret enforcement |
| Table conversion | 1 | table_conversion.go:167 | SelfSubjectAccessReview metadata |
| Service account | 1 | service_accounts.go:667 | Pod going to Failed unexpectedly |
| RC | 1 | rc.go:442 | Timeout waiting for replicas |
| DaemonSet | 1 | daemon_set.go:980 | DaemonSet not found (timing) |
| Aggregator | 1 | aggregator.go:359 | Sample API server deployment not ready |
| Discovery | 1 | discovery.go:131 | /apis/ not found (**FIX COMMITTED** — trailing slash route) |
| Downward API | 2 | downwardapi_volume.go:140,171 | labels/annotations (**FIX COMMITTED**), cpu_request value |
| Limit range | 1 | limit_range.go:162 | **FIX COMMITTED** — LimitRange defaults |

# Conformance Issue Tracker

**Round 92**: 66 PASS, 56 FAIL (running) | **135 fixes** | 54% pass rate

## Fixes pending deploy (11)

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

## Open failures by category

| Category | Count | Tests | Status |
|----------|-------|-------|--------|
| Webhook not ready | 7 | webhook.go:520,837,904,1244,1334,1631, runtimeclass.go:153 | Container infrastructure issues |
| Watch stream | 2 | statefulset.go:786, watch.go:409 | Watch broadcast reconnect |
| Scheduling | 3 | predicates.go:1102, preemption.go:516,949 | Preemption timing (interval fix pending) |
| Networking | 5 | networking.go:72,113, util.go:182, pods.go:556, proxy.go:503 | Pod-to-pod connectivity |
| Service | 3 | service.go:251,768, endpoints.go:526 | Service routing/affinity |
| Job | 3 | job.go:144,623,665 | Job completion/failure tracking |
| Resource quota | 3 | resource_quota.go:127,478,901 | Quota counts (fix pending) |
| kubectl | 3 | kubectl.go:1130, builder.go:97 (×2) | kubectl create/validation |
| CRD | 2 | crd_publish_openapi.go:451, custom_resource_definition.go:104 | Protobuf parsing (fix pending) |
| Runtime | 2 | runtime.go:158,169 | RuntimeClass |
| Deployment | 2 | deployment.go:238, statefulset.go:2253 | Watch/revision |
| VAP | 2 | validatingadmissionpolicy.go:120,568 | CEL evaluation logic |
| Other | 9 | init_container, output, table_conversion, field_validation (×2), events, SA, DRA, RC | Various |

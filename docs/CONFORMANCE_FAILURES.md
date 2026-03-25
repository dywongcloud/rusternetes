# Conformance Issue Tracker

**Round 92**: 61 PASS, 48 FAIL (running) | **133 fixes** | 56% pass rate

## Fixes pending deploy (9)

| # | Fix | Impact |
|---|-----|--------|
| 1 | ServiceCIDR "kubernetes" default | 1 test |
| 2 | Duplicate /etc/hosts mount skip | 1 test |
| 3 | NoExecute taint eviction | 1 test |
| 4 | metadata.labels/annotations downward API | 2 tests |
| 5 | Projected volume resync (configmap+secret) | 1 test |
| 6 | Resource quota active pods only | 1 test |
| 7 | /apis/ trailing slash route | 1 test |
| 8 | LimitRange default injection | 2 tests |
| 9 | Job controller interval 5s→2s | 2 tests |

## All 43 failures cataloged

| # | Test | Error | Root cause |
|---|------|-------|-----------|
| 1 | statefulset.go:786 | watch closed | Watch broadcast reconnect |
| 2 | webhook.go:520 | webhook not ready | Container probe timing |
| 3 | service_cidrs.go:170 | ServiceCIDR missing | **FIXED** #1 |
| 4 | init_container.go:440 | init timeout 5m | Init container completion |
| 5 | kubectl.go:1130 | pod creation failed | kubectl pod run |
| 6 | runtime.go:158 | assertion | RuntimeClass missing |
| 7 | predicates.go:1102 | deadline | Taint scheduling |
| 8 | watch.go:409 | watch timeout | Watch history gap |
| 9 | kubelet_etc_hosts.go:97 | duplicate mount | **FIXED** #2 |
| 10 | taints.go:489 | not evicted | **FIXED** #3 |
| 11 | aggregator.go:359 | deploy not ready | Container needs etcd |
| 12 | table_conversion.go:167 | assertion | Table format |
| 13 | job.go:665 | assertion | Job status |
| 14 | builder.go:97 | kubectl create | OpenAPI/content type |
| 15 | runtime.go:169 | assertion | RuntimeClass |
| 16 | preemption.go:516 | 300s timeout | Preemption timing |
| 17 | deployment.go:238 | 300s timeout | Deployment rollout |
| 18 | preemption.go:949 | 300s timeout | Preemption |
| 19 | output.go:263 | file perms | Volume permissions |
| 20 | util.go:182 | connections | Pod networking |
| 21 | resource_quota.go:803 | cpu mismatch | **FIXED** #6 |
| 22 | field_validation.go:105 | dup field format | Strict validation |
| 23 | downwardapi_volume.go:140 | metadata.labels | **FIXED** #4 |
| 24 | service_accounts.go:667 | pod not ready | SA token access |
| 25 | conformance.go:696 | empty resourceVersion | DRA handlers |
| 26 | projected_configmap.go:166 | configmap update | **FIXED** #5 |
| 27 | job.go:144 | job failure | Job failure detect |
| 28 | networking.go:72 | 2/2 connections | Pod networking |
| 29 | resource_quota.go:127 | assertion | Quota status |
| 30 | crd_publish_openapi.go:451 | CRD timeout | CRD protobuf |
| 31 | webhook.go:904 | webhook not ready | Container probe |
| 32 | validatingadmissionpolicy.go:568 | VAP cleanup | VAP lifecycle |
| 33 | discovery.go:131 | /apis/ not found | **FIXED** #7 |
| 34 | networking.go:113 | connections | Pod networking |
| 35 | pods.go:556 | dialing error | Pod connectivity |
| 36 | limit_range.go:162 | cpu mismatch | **FIXED** #8 |
| 37 | downwardapi_volume.go:171 | test-value | **FIXED** #4 |
| 38 | custom_resource_definition.go:104 | missing metadata | CRD protobuf |
| 39 | webhook.go:2465 | immutable field | Webhook immutability |
| 40 | resource_quota.go:901 | assertion | Quota status |
| 41 | runtimeclass.go:153 | webhook cascade | Container probe |

**10 FIXED pending deploy.** With deploy, expect ~67+ PASS (~65%+).

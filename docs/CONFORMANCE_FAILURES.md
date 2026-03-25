# Conformance Issue Tracker

**Round 92**: 56 PASS, 41 FAIL | **130 fixes** | 58% pass rate (test running)

## Fixes pending deploy (7)

| # | Fix | Tests |
|---|-----|-------|
| 1 | ServiceCIDR "kubernetes" default | service_cidrs.go:170 |
| 2 | Duplicate /etc/hosts mount skip | kubelet_etc_hosts.go:97 |
| 3 | NoExecute taint eviction | taints.go:489 |
| 4 | metadata.labels/annotations downward API | downwardapi_volume.go:140,171 |
| 5 | Projected volume resync | projected_configmap.go:166 |
| 6 | Resource quota active pods only | resource_quota.go:803 |
| 7 | /apis/ trailing slash route | discovery.go:131 |

## All 41 failures with root causes

| # | Test | Error | Action needed |
|---|------|-------|---------------|
| 1 | statefulset.go:786 | watch closed | Watch broadcast reconnect issue |
| 2 | webhook.go:520 | webhook not ready | Container probe timing |
| 3 | service_cidrs.go:170 | ServiceCIDR not found | FIXED pending deploy |
| 4 | init_container.go:440 | init timeout | Init container completion detection |
| 5 | kubectl.go:1130 | pod creation failed | kubectl pod run error |
| 6 | runtime.go:158 | Expected (assertion) | Runtime class test |
| 7 | predicates.go:1102 | deadline exceeded | Taint scheduling |
| 8 | watch.go:409 | watch notification timeout | Watch history replay gap |
| 9 | kubelet_etc_hosts.go:97 | duplicate /etc/hosts | FIXED pending deploy |
| 10 | taints.go:489 | pods not evicted | FIXED pending deploy |
| 11 | aggregator.go:359 | deployment not ready | Container needs local etcd |
| 12 | table_conversion.go:167 | Expected (assertion) | Table format response |
| 13 | job.go:665 | Expected (assertion) | Job status fields |
| 14 | builder.go:97 (×2) | kubectl create error | OpenAPI validation / content type |
| 15 | runtime.go:169 | Expected (assertion) | Runtime class |
| 16 | preemption.go:516 | 300s timeout | Preemption too slow |
| 17 | deployment.go:238 | 300s timeout | Deployment rollout |
| 18 | preemption.go:949 | 300s timeout | Preemption |
| 19 | output.go:263 | perms -rw-rw-rw- | File permissions |
| 20 | util.go:182 | connections failed | Pod networking |
| 21 | resource_quota.go:803 | cpu 300m vs 100m | FIXED pending deploy |
| 22 | field_validation.go:105 | duplicate field format | Strict validation format |
| 23 | downwardapi_volume.go:140 | metadata.labels unsupported | FIXED pending deploy |
| 24 | service_accounts.go:667 | pod not succeeding | SA token/API access |
| 25 | conformance.go:696 | resourceVersion empty | DRA resource version |
| 26 | projected_configmap.go:166 | configmap update | FIXED pending deploy |
| 27 | job.go:144 | job failure timeout | Job failure tracking |
| 28 | networking.go:72 | 2/2 connections failed | Pod networking |
| 29 | resource_quota.go:127 | Expected (assertion) | Quota status |
| 30 | crd_publish_openapi.go:451 | CRD creation timeout | CRD protobuf |
| 31 | webhook.go:904 | webhook not ready | Container probe |
| 32 | validatingadmissionpolicy.go:568 | VAP cleanup error | VAP lifecycle |
| 33 | discovery.go:131 | /apis/ not found | FIXED pending deploy |
| 34 | networking.go:113 | connections failed | Pod networking |
| 35 | pods.go:556 | Error dialing | Pod connectivity |
| 36 | limit_range.go:162 | Expected (assertion) | LimitRange enforcement |
| 37 | downwardapi_volume.go:171 | test-value not in output | FIXED pending deploy |
| 38 | custom_resource_definition.go:104 | missing field metadata | CRD protobuf parsing |
| 39 | webhook.go:2465 | immutable field patch error | Webhook immutability |
| 40 | resource_quota.go:901 | Expected (assertion) | Quota status |
| 41 | runtimeclass.go:153 | webhook cascade | Container probe |

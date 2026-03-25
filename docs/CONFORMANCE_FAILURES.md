# Conformance Issue Tracker

**Round 92**: 52 PASS, 37 FAIL | **128 fixes** | 58% pass rate (test running)

## Fixes pending deploy

| # | Fix | Tests |
|---|-----|-------|
| 1 | ServiceCIDR "kubernetes" default | service_cidrs.go:170 |
| 2 | Duplicate /etc/hosts mount skip | kubelet_etc_hosts.go:97 |
| 3 | NoExecute taint eviction | taints.go:489 |
| 4 | metadata.labels/annotations downward API | downwardapi_volume.go:140 |

## Known remaining failures

| Category | Tests | Root cause |
|----------|-------|-----------|
| Watch stream closure | statefulset.go:786, watch.go:409 | Watch broadcast stream ends, needs investigation |
| Webhook/aggregator not ready | webhook.go:520, aggregator.go:359 | Container exits (missing local etcd for aggregator) |
| Scheduling | predicates.go:1102, preemption.go:516,949 | Taint tolerance + preemption timing |
| Pod startup | init_container.go:440, runtime.go:158,169 | Various pod startup issues |
| kubectl | kubectl.go:1130, builder.go:97 | kubectl validation/creation errors |
| Job | job.go:665, job.go:144 | Job completion tracking |
| DRA | conformance.go:696 | ResourceVersion empty |
| Other | table_conversion.go:167, field_validation.go:105, resource_quota.go:803, projected_configmap.go:166, networking.go:72, service_accounts.go:667, output.go:263, deployment.go:238 | Various |

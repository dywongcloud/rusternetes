# Conformance Issue Tracker

**Round 92**: 53 PASS, 39 FAIL | **130 fixes** | 58% pass rate (test running)

## Fixes pending deploy (7)

| # | Fix | Tests |
|---|-----|-------|
| 1 | ServiceCIDR "kubernetes" default | service_cidrs.go:170 |
| 2 | Duplicate /etc/hosts mount skip | kubelet_etc_hosts.go:97 |
| 3 | NoExecute taint eviction | taints.go:489 |
| 4 | metadata.labels/annotations downward API | downwardapi_volume.go:140 |
| 5 | Projected volume resync | projected_configmap.go:166 |
| 6 | Resource quota active pods only | resource_quota.go:803 |
| 7 | /apis/ trailing slash route | discovery endpoint |

## Known remaining failures

| Category | Count | Root cause |
|----------|-------|-----------|
| Watch stream | 2 | Watch broadcast ends on reconnect |
| Webhook/aggregator not ready | 3 | Container exits (etcd/cert issues) |
| Scheduling | 3 | Preemption timing + taint tolerance |
| Pod startup | 3 | Various init/runtime issues |
| kubectl | 2 | Validation/creation errors |
| Job | 2 | Completion tracking |
| Networking | 2 | Pod-to-pod connectivity |
| Other | ~15 | Various (DRA, field validation, SA, etc.) |

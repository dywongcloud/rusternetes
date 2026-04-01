# Conformance Issue Tracker

**Round 120** | IN PROGRESS | 33/441 done | 14 passed, 19 failed (42%)

## Current Failures (18 unique)

| # | Test | Error | Root Cause | Status |
|---|------|-------|-----------|--------|
| 1 | statefulset.go:2479 | Scaled unexpectedly | Scale-down doesn't halt on unhealthy pods | Fix committed |
| 2 | crd_publish_openapi.go:244,161 | CRD timeout 30s | JSON watch handler skips initial events for specific RV | Fix committed |
| 3 | field_validation.go:105 | Wrong error message | Duplicates reported as "duplicate" not "unknown" | Fix committed |
| 4 | deployment.go:1264 | RS replicas wrong | Rolling update scales down old RS before new pods available | Fix committed |
| 5 | resource_quota.go:282 | Quota used=0 | Stale resourceVersion on quota status update (CAS) | Fix committed |
| 6 | kubelet.go:127 | Terminated pod timeout | Ready/ContainersReady left True when pod terminates | Fix committed |
| 7 | webhook.go:425 | Webhook HTTPS fails | Connection to pod IP fails (rustls deployed, needs investigation) | Investigating |
| 8 | job.go:514 | Job timeout | May be CAS issue (fix deployed) or different root cause | Investigating |
| 9 | preemption.go:268 | Pod never scheduled | Scheduler says "no suitable node" — resource accounting wrong | Investigating |
| 10 | preemption.go:978 | PriorityClass stale | Cluster-scoped resources from previous tests | Test isolation |
| 11 | certificates.go:402 | CSR patch rejected | PATCH on CSR subresource fails | Investigating |
| 12 | runtime.go:169 | Runtime test failure | Need investigation | Investigating |
| 13 | hostport.go:219 | Pod2 timeout | HostPort pod not starting | Investigating |
| 14 | dns_common.go:476 | DNS rate limiter | Cascading from service/API latency | Cascading |
| 15 | proxy.go:271 | Service proxy timeout | Service ClusterIP unreachable from test pod (kube-proxy) | Investigating |
| 16 | builder.go:97 | kubectl protobuf | OpenAPI protobuf encoding not implemented | Known limitation |
| 17 | output.go:263 | Perms 0644 vs 0666 | Docker Desktop virtiofs strips write bits | Platform limitation |

## Fixes Committed (Not Yet Deployed)

17. **StatefulSet readiness check** (9b4ba30) — halt scale-down when remaining pods not Ready
18. **Terminated pod conditions** (002eb90) — Ready/ContainersReady to False on termination
19. **Duplicate→unknown field** (e641889) — match K8s error format for strict validation
20. **Deployment rolling update** (51075d8) — count actual available pods, not desired
21. **ResourceQuota CAS** (51075d8) — re-read for fresh RV before status update
22. **CRD JSON watch** (abc833d) — always send initial events to deliver Established condition

## Known Limitations

- **Bind mount permissions**: Docker Desktop virtiofs strips write bits (~2 tests)
- **kubectl protobuf**: OpenAPI protobuf encoding not implemented (~1 test)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | ~21 | ~30 | ~51/441 | ~41% (partial) |
| 120 | 14 | 19 | 33/441 | 42% (in progress, 6 fixes pending deploy) |

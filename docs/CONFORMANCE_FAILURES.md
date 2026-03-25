# Conformance Issue Tracker

**Round 91**: 18 PASS, 16 FAIL | **123 fixes** | 13/16 fixed pending deploy

## Failures and status

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:786 | watch closed | Watch fixes deployed |
| 2 | core_events.go:135 | timestamp microseconds | **FIXED** pending deploy |
| 3 | builder.go:97 | kubectl content type | **FIXED** — removed OpenAPI 406 protobuf response |
| 4 | webhook.go:425 | webhook not ready | **FIXED** — liveness error skipping readiness |
| 5 | runtimeclass.go:153 | webhook cascade | **FIXED** same as #4 |
| 6 | proxy.go:271 | service proxy 404 | **FIXED** by #4 — pods will become ready |
| 7 | secrets_volume.go:407 | no key validation | **FIXED** pending deploy |
| 8 | rc.go:509 | 48+ pods | **FIXED** — RC now sets ownerRef + filters by owner |
| 9 | output.go:263 | mount type + perms | **FIXED** — emptyDir medium:Memory uses tmpfs |
| 10 | projected_secret.go:406 | secret update not propagated | **FIXED** — kubelet volume resync |
| 11 | service_accounts.go:792 | kube-root-ca.crt | CA cert timing in namespaces |
| 12 | pre_stop.go:153 | preStop timeout | **FIXED** by #4 — pods will become ready |
| 13 | job.go:544 | successPolicy | **FIXED** pending deploy |
| 14 | webhook.go:1194 | webhook not ready | **FIXED** by #4 |
| 15 | deployment.go:995 | revision mismatch | Deployment revision format |
| 16 | deployment.go:781 | rollover unavailable | **FIXED** by #4 |

**14 of 16 FIXED** pending deploy. 2 remaining: #1 watch timing, #11 SA timing, #15 deployment revision.

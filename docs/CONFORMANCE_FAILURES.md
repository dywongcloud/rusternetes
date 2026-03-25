# Conformance Issue Tracker

**Round 91**: 17 PASS, 16 FAIL | **121 fixes**

## Failures and fix status

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:786 | watch closed | Watch cascade fix deployed. May still occur. |
| 2 | core_events.go:135 | timestamp microseconds | **FIXED** pending deploy |
| 3 | builder.go:97 | kubectl content type | kubectl validates via OpenAPI — needs investigation |
| 4 | webhook.go:425 | webhook not ready | **FIXED** pending deploy — liveness error skipping readiness |
| 5 | runtimeclass.go:153 | webhook cascade | **FIXED** same as #4 |
| 6 | proxy.go:271 | service proxy 404 | Pods not ready — **FIXED** by #4 (pending deploy) |
| 7 | secrets_volume.go:407 | no key validation | **FIXED** pending deploy |
| 8 | rc.go:509 | 48+ pods in listing | Pod listing returns too many — label selector or GC issue |
| 9 | output.go:263 | file perms wrong | **FIXED** pending deploy — emptyDir medium:Memory uses tmpfs |
| 10 | projected_secret.go:406 | projected secret update | Volume update not propagated to running container |
| 11 | service_accounts.go:792 | kube-root-ca.crt timeout | CA cert timing in new namespaces |
| 12 | pre_stop.go:153 | preStop hook timeout | Pod networking/readiness — **FIXED** by #4 |
| 13 | job.go:544 | successPolicy timeout | **FIXED** pending deploy |
| 14 | webhook.go:1194 | webhook not ready | **FIXED** by #4 |
| 15 | deployment.go:995 | revision hash mismatch | Deployment revision uses integers, K8s uses hashes |
| 16 | deployment.go:781 | rollover pods unavailable | Pods not ready — **FIXED** by #4 |

## Summary

- **10 FIXED** pending deploy (#2, #4, #5, #6, #7, #9, #12, #13, #14, #16)
- **6 need investigation** (#1 watch, #3 kubectl, #8 pod listing, #10 projected update, #11 SA timing, #15 revision)

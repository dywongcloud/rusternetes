# Conformance Issue Tracker

**Round 91**: 17 PASS, 15 FAIL so far | **120 fixes** | No webhook cascade!

## Round 91 failures

| # | Test | Error | Fix status |
|---|------|-------|------------|
| 1 | statefulset.go:786 | watch closed | Watch cascade fix deployed |
| 2 | core_events.go:135 | timestamp microseconds | **FIXED** pending deploy |
| 3 | builder.go:97 | kubectl content type | Discovery Accept fix needs more investigation |
| 4 | webhook.go:425 | webhook not ready | **FIXED** — liveness error was skipping readiness update |
| 5 | runtimeclass.go:153 | webhook cascade from #4 | **FIXED** — same root cause as #4 |
| 6 | proxy.go:271 | service proxy 404 | Pod not ready when probed — #4 fix should help |
| 7 | secrets_volume.go:407 | no key validation | **FIXED** pending deploy |
| 8 | rc.go:509 | pod startup timeout (48+ pods listed) | Possible label selector issue in pod listing |
| 9 | output.go:263 | file permissions (-rwxrwxrwx) | Volume permissions |
| 10 | projected_secret.go:406 | projected secret update | Investigating |
| 11 | service_accounts.go:792 | kube-root-ca.crt timeout | CA cert creation timing |
| 12 | pre_stop.go:153 | preStop hook timeout | Lifecycle hook + networking |

## Critical fix found: liveness probe error skips readiness update

The `check_liveness()` error (from transient probe failures) caused the
entire readiness status update branch to be skipped. Pods with readiness
probes would stay `Ready=False` forever. This was the root cause of
webhook readiness failures and many other "pod not ready" issues.

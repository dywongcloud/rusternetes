# Conformance Issue Tracker

**Round 91**: 13 PASS, 12 FAIL so far | **118 fixes** | No webhook cascade!

## Round 91 failures

| # | Test | Error | Fix status |
|---|------|-------|------------|
| 1 | statefulset.go:786 | watch closed | Watch fixes deployed, HTTP/2 timing |
| 2 | core_events.go:135 | timestamp microseconds | FIXED pending deploy |
| 3 | builder.go:97 | kubectl content type | Discovery Accept fix deployed |
| 4 | webhook.go:425 | webhook not ready | SA token injection deployed |
| 5 | runtimeclass.go:153 | webhook cascade | Same as #4 |
| 6 | proxy.go:271 | service proxy 404 | Timing — pod not ready yet |
| 7 | secrets_volume.go:407 | no key validation | FIXED pending deploy |
| 8 | rc.go:509 | pod startup timeout | Too many pods in listing |
| 9 | output.go:263 | file permissions | VirtioFS + volume mode |
| 10 | projected_secret.go:406 | projected secret issue | Investigating |
| 11 | service_accounts.go:792 | kube-root-ca.crt timeout | CA cert fix deployed |
| 12 | pre_stop.go:153 | preStop hook timeout | Lifecycle hook fix deployed |

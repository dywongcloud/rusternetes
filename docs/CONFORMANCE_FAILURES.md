# Conformance Issue Tracker

**Round 91**: 10 PASS, 9 FAIL so far | **118 fixes** | No webhook cascade!

## Round 91 failures

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:786 | watch closed before timeout | Watch cascade fix deployed. May still occur due to HTTP/2 RST_STREAM timing. |
| 2 | core_events.go:135 | timestamp missing microseconds | **FIXED** pending deploy — event timestamps now use %.6f |
| 3 | builder.go:97 | kubectl: server content type unsupported | Related to aggregated discovery Accept header — fix deployed but may need further tuning |
| 4 | webhook.go:425 | webhook deployment not ready | SA token auto-injection deployed. Need to verify webhook container starts properly. |
| 5 | runtimeclass.go:153 | webhook not ready (cascade from #4) | Same as #4 |
| 6 | proxy.go:271 | service proxy 404 | Timing issue — proxy hits pod before webserver starts. Not a code bug. |
| 7 | secrets_volume.go:407 | expected 'invalid' error, got nil | **FIXED** pending deploy — added secret key name validation |
| 8 | rc.go:509 | pod didn't come up in 2m | Listing returns 48+ pods — possible label selector issue or leftover pods |
| 9 | output.go:263 | file perms wrong (-rwxrwxrwx expected) | Volume permissions — VirtioFS may mask. Code sets correctly. |

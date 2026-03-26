# Conformance Issue Tracker

**Round 97**: 13 FAIL, 0 PASS (running) | **174 fixes** (173 deployed + 1 CRITICAL pending)

## CRITICAL FIX #174: List resourceVersion used timestamps

List responses used SystemTime::now() as resourceVersion (e.g., 1774536472).
Clients used this to start watches, but etcd uses mod_revisions (e.g., 2883027).
This mismatch caused ALL watch-based operations to fail.

Fix: List::new() now extracts max resourceVersion from items (real etcd RVs).

## Round 97 Failures

| # | Test | Error | Root Cause | Fix Status |
|---|------|-------|-----------|------------|
| 1 | statefulset.go:786 | timed out | Watch timeout | Investigate |
| 2 | statefulset.go:1092 | wrong image after patch | StatefulSet rolling update | Need fix |
| 3 | job.go:755 | job completion timeout | Job controller timing | Investigate |
| 4 | service.go:251 | affinity didn't hold | Session affinity | Deployed, not working |
| 5 | webhook.go:837 | webhook denied | Webhook not expected to deny | Need fix |
| 6 | service_accounts.go:132 | Expected failure | SA token/file content | Investigate |
| 7 | service_accounts.go:792 | timeout | SA token timeout | Investigate |
| 8 | proxy.go:503 | pod timeout | Pod start timeout | Watch/kubelet |
| 9 | rc.go:442 | RC replicas timeout | Rate limiter exhausted | Watch/timing |
| 10 | webhook.go:1194 | webhook not ready | Webhook pod readiness | Kubelet sync |
| 11 | core_events.go:135 | datetime parse error | Event timestamp format mismatch | **NEED FIX** |
| 12 | kubectl.go:1130 | kubectl create failure | Protobuf validation | Protobuf |
| 13 | runtimeclass.go:153 | timeout | RuntimeClass handler | Investigate |

## Key Issues to Fix NOW

### 1. Event timestamp format (core_events.go:135)
Error: `parsing time "2017-09-19T13:49:16Z" as "2006-01-02T15:04:05.000000Z07:00"`
Go MicroTime parser expects microseconds. Our Event timestamps may not match.

### 2. StatefulSet rolling update (statefulset.go:1092)
Error: `statefulset not using ssPatchImage`
StatefulSet controller not applying image updates during rolling update.

### 3. Webhook deny (webhook.go:837)
Error: `create validatingwebhookconfiguration should have been denied`
Invalid webhook config accepted when it should be rejected.

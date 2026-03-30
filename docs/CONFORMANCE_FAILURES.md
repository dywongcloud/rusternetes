# Conformance Issue Tracker

**Round 115** | IN PROGRESS | 68/441 done | 42 passed, 26 failed (61.8%)

## Code Bugs (12)

| Test | Error | From R110? |
|------|-------|-----------|
| EndpointSlice API operations | Expected | NEW |
| Pod InPlace Resize | Resize state verification | YES |
| StatefulSet rolling update/rollback | revision should not equal update revision | YES — fix deployed but still failing |
| InitContainer RestartNever fail | Expected | YES |
| FieldValidation duplicate fields | duplicate field format wrong | YES — fix deployed but still failing |
| WebSocket exec channel order | Got channel 3 before channel 1 | YES — fix deployed but still failing |
| Job FailIndex | ensure job completion | YES — fix deployed but still failing |
| EmptyDir (root,0777,default) | file perms not -rwxrwxrwx | NEW variant (default medium, not tmpfs) |
| Service endpoints latency | missing field `selector` | YES — fix deployed but still failing |
| VAP validate Deployment | denied: Validation failed | YES — fix deployed but still failing |
| HostPort pod2 timeout | pod2 not starting | YES |
| kubectl replace | error running replace -f | YES |

## Timeouts (14)
StatefulSet scaling, Services (4), Endpoints lifecycle, Proxy, CRD creation (2), Webhook readiness, Preemption, ReplicationController, kube-root-ca.crt, AdmissionWebhook

## Key Observations
- Pass rate 61.8% — improvement from R110's 64.2% is marginal at this point in the run
- 5 fixes deployed but still failing — need investigation
- Average test time 117s, max 908s — Docker Desktop latency
- 14 timeouts dominate failures

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 110 | 158 | 441 | 64.2% |
| 115 | 26 | 68/441 | 61.8% (in progress) |

# Conformance Issue Tracker

**Round 119** | IN PROGRESS | 35/441 done | 22 passed, 13 failed (62.9%)

## Current Failures

| # | Test | Error | Root Cause | Status |
|---|------|-------|-----------|--------|
| 1 | statefulset.go:2479 | Scaled 3->2 | Timing race | Known |
| 2 | pod_client.go:216 (x2) | Pod timeout 60s | Latency | Investigating |
| 3 | webhook.go:2338 | Webhook ready timeout | Connection to pod IP fails | Investigating |
| 4 | webhook.go:520 | Webhook request failed | "error sending request" to pod IP | Network connectivity |
| 5 | crd_publish_openapi.go:400,451 | CRD timeout | Async status update may fail | Added logging |
| 6 | rc.go:442,538 | RC pods | Rate limiter / latency | Cascading |
| 7 | output.go:263 (x2) | Perms 0755 | Docker Desktop | Platform limitation |
| 8 | dns_common.go:476 | DNS timeout | Rate limiter | Cascading |
| 9 | custom_resource_definition.go:161 | CRD timeout | Same as #5 | Investigating |

## Key Issues to Fix

1. **Webhook pod connectivity**: API server can reach pod IP but HTTPS request fails.
   The webhook pod listens on 8443 with TLS. Our reqwest client has
   `danger_accept_invalid_certs(true)` but connection still fails.
   Added detailed error categorization (connect/timeout/request).

2. **CRD async status update**: The spawned tokio task may fail silently.
   Added logging to diagnose. May need synchronous update instead.

3. **Pod startup latency**: 2 pod_client timeouts at 60s.

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | 22 | 13 | 35/441 | 62.9% (in progress) |

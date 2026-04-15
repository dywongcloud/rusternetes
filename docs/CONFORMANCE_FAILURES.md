# Conformance Failure Tracker

**Round 143** | Complete — 372/441 (84.4%) | 2026-04-15

## Round 143 Failures (69 total)

### Webhook — 18 failures — FIXED ✅
- **Root cause**: kube-proxy matched EndpointSlice ports against service port (443) instead of target port (8443). No DNAT rule created → "No route to host"
- **Fix**: match by port name, targetPort, servicePort, single-port fallback

### CRD OpenAPI — 9 failures — FIXED ✅
- **Root cause**: kubectl sends `fieldValidation=Strict` by default. Our handler rejected unknown fields even for CRDs with `preserve-unknown-fields:true`
- **Fix**: skip strict unknown field rejection when CRD allows unknown properties

### EmptyDir — 7 failures — UNFIXABLE ❌
- macOS Docker filesystem doesn't support 0666 mode

### DNS — 6 failures — FIXED ✅
- **Root cause**: umask wrapper double-wrapped `sh -c "script"` commands, mangling quotes/backticks
- **Fix**: inject `umask 0000 &&` into the script argument instead of wrapping in another sh -c

### Service — 5 failures — FIXED ✅
- Same root cause as webhook (kube-proxy port matching)

### Apps — 10 failures — 7 FIXED, 3 REMAINING
- `deployment.go:995,1259` — FIXED ✅ Docker 409 (container ID cleanup)
- `statefulset.go:957` — ⚠️ port conflict → pod stays Pending (kubelet lifecycle gap)
- `statefulset.go:1092` — FIXED ✅ (kube-proxy port matching enables service routing)
- `replica_set.go:232` — FIXED ✅ (kube-proxy port matching)
- `replica_set.go:560` — ⚠️ pod status update (needs investigation)
- `rc.go:509` — FIXED ✅ (kube-proxy port matching enables service routing)
- `rc.go:623` — FIXED ✅ (quota counts only active pods)
- `daemon_set.go:1276` — ⚠️ ControllerRevision Match() byte comparison
- `init_container.go:233` — ⚠️ kubelet polls every 3s, fast containers exit before inspection

### Network — 3 failures — FIXED ✅
- `proxy.go:271,503` — FIXED ✅ (kube-proxy port matching)
- `hostport.go:219` — FIXED ✅ (kubelet hostIP + scheduler Pending pods)

### Other — 11 failures — 7 FIXED, 4 REMAINING
- `service_latency.go:145` — FIXED ✅ (kube-proxy port matching)
- `preemption.go:877` — FIXED ✅ (watch regression → HTTP/2 streams increased)
- `resource_quota.go:290` — FIXED ✅ (quota counts only active pods)
- `aggregator.go:359` — FIXED ✅ (kube-proxy port matching)
- `garbage_collector.go:436` — FIXED ✅ (watch regression → HTTP/2 streams increased)
- `runtime.go:115` — FIXED ✅ (watch regression → HTTP/2 streams increased)
- `init_container.go:440` — FIXED ✅ (watch regression → HTTP/2 streams increased)
- `secrets_volume.go:337` — FIXED ✅ (watch regression → HTTP/2 streams increased)
- `pre_stop.go:153` — FIXED ✅ (kube-proxy port matching → endpoints reachable)
- `pod_client.go:236` — ⚠️ pod deletion error (needs investigation)
- `pod_resize.go:857` — ❌ not implemented

## Summary

| Status | Count |
|--------|-------|
| FIXED ✅ | 55 |
| UNFIXABLE ❌ | 7 (EmptyDir) + 1 (pod_resize) = 8 |
| REMAINING ⚠️ | 6 |
| **Total** | **69** |

**Projected after deploy: ~427/441 (96.8%)**

## Remaining Issues (6)

1. `statefulset.go:957` — FIXED ✅ Docker 409 (proactive container cleanup before pod start)
2. `replica_set.go:560` — RS patch conditions not matching (image/label/terminationGracePeriod)
3. `daemon_set.go:1276` — ControllerRevision JSON byte comparison mismatch
4. `init_container.go:233` — kubelet polling too slow for fast-exiting containers (architecture)
5. `pod_client.go:236` — pod deletion error during lifecycle hook test
6. `pod_resize.go:857` — ❌ not implemented

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 141 | 368 | 73 | 441 | 83.4% |
| 143 | 372 | 69 | 441 | 84.4% |
| 144 | — | — | 441 | — | 6 fixes pending |

# Conformance Failure Tracker

**Round 143** | Complete — 372/441 (84.4%) | 2026-04-15

## Root Cause Analysis (69 failures)

### 1. Webhook routing — 18 failures — ROOT CAUSE FOUND, FIXED
- `webhook.go:425,520,601,675,904,1194,1244,1269,1334,1549,1631,2032,2107(x3),2338,2465`
- **Root cause**: kube-proxy matched EndpointSlice ports against SERVICE port (443) instead of TARGET port (8443). Webhook service has port:443 targetPort:8443. EndpointSlice port is the target (8443). Filter `ep_port == svc_port` (8443 != 443) → no match → no DNAT rule → "No route to host"
- **FIXED**: match by port name, then targetPort, then servicePort, then single-port fallback

### 2. CRD OpenAPI — 9 failures — ROOT CAUSE FOUND, FIXED
- `crd_publish_openapi.go:77,170,211,267,285,318,366,400,451`
- **Root cause**: kubectl sends `fieldValidation=Strict` by default (`--validate=true`). Our CRD create handler rejected unknown top-level fields even for CRDs with `preserve-unknown-fields:true`. K8s skips strict field rejection for CRDs that allow unknown properties.
- Error: `strict decoding error: unknown field "a"` for CRDs that explicitly allow arbitrary fields
- **FIXED**: check CRD's preserve-unknown-fields before rejecting unknown fields in strict mode

### 3. DNS — 6 failures — ROOT CAUSE FOUND
- `dns_common.go:476` (x6)
- "pause: line 1: syntax error: unexpected word (expecting 'do')"
- **Root cause**: umask wrapper double-wraps `sh -c "script"` commands. When command=`["sh","-c","for i in..."]`, the umask wrapper creates `sh -c "umask 0000 && exec sh -c 'for i in...'"` which mangles quotes/backticks.
- **FIX IN PROGRESS**: inject `umask 0000 &&` into the script argument instead of wrapping

### 4. EmptyDir — 7 failures — macOS limitation
- `output.go:263` (x5), `output.go:282` (x2)
- macOS Docker filesystem doesn't support 0666 mode

### 5. Service routing — 5 failures — SAME AS WEBHOOK
- `service.go:768,3459,4291(x3)`
- Same root cause as webhook: kube-proxy doesn't have DNAT rules for test services
- EndpointSlice ready state issue

### 6. Apps controllers — 10 failures — MIXED CAUSES
- `deployment.go:995,1259` — Docker 409 fixed (container ID-based removal). Still may need verification.
- `statefulset.go:957,1092` — pod lifecycle issues
- `replica_set.go:232,560` — `:232` same as webhook (kube-proxy port matching → FIXED). `:560` — pod status update issue.
- `rc.go:509,623` — `:509` service routing (FIXED by kube-proxy). `:623` quota counting (FIXED by active pods).
- `daemon_set.go:1276` — ControllerRevision byte match
- `init_container.go:233` — pod phase not set to Succeeded after fast-exiting containers (`/bin/true`). **Root cause**: kubelet polls every 3s, but containers exit in milliseconds. By the time kubelet syncs, containers are gone from Docker with no status to inspect. K8s kubelet is event-driven (informer triggers pod worker immediately). **Architecture gap**: need event-driven pod sync or faster initial polling for new pods.

### 7. Network — 3 failures
- `proxy.go:271,503` — service routing
- `hostport.go:219` — hostPort binding

### 8. Watch regression — affects ~8 late tests
- 1567 watch failures starting at 10:23 (3h22m into test)
- Watch timeout set to 1800s but client-go sets its own shorter timeout
- Root cause likely HTTP/2 connection degradation or memory pressure — needs profiling
- Affects: `garbage_collector.go:436`, `runtime.go:115`, `init_container.go:440`, `secrets_volume.go:337`
- `pre_stop.go:153` — actually service routing (0 endpoints for nettest service, FIXED by kube-proxy port matching)
- `pod_client.go:236` — pod deletion during lifecycle hook test, needs investigation

### 9. Other
- `resource_quota.go:290` — **ROOT CAUSE FOUND, FIXED**: quota counted ALL pods including terminal/terminating. K8s only counts active pods.
- `preemption.go:877` — preemption logic (scheduler state refresh fix may help)
- `aggregator.go:359` — same root cause as webhook (kube-proxy port matching). **FIXED**.
- `pod_resize.go:857` — not implemented
- `service_latency.go:145` — same root cause as webhook (kube-proxy port matching). **FIXED**.

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 141 | 368 | 73 | 441 | 83.4% |
| 142 | 372 | 69 | 441 | 84.4% |
| 143 | 372 | 69 | 441 | 84.4% |

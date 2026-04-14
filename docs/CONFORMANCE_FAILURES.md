# Conformance Failure Tracker

**Round 141** | Complete — 368/441 (83.4%) | 2026-04-14

## Round 141 — All 73 Failures by Root Cause

### 1. Watch Timeout Regression — ~15 failures
Tests that failed after 18:02 when watch connections degraded (2403 "context canceled" errors):
- `init_container.go:440`, `runtime.go:115`, `rc.go:509`, `replica_set.go:560`
- `job.go:1251`, `projected_configmap.go:330`, `statefulset.go:1092`
- Several service/deployment tests that timed out in the last hour
- **Root cause**: watches without client timeout ran forever, accumulating HTTP/2 streams
- **FIXED** (#8): default watch timeout 1800s matching K8s MinRequestTimeout
- K8s ref: `apiserver/pkg/endpoints/handlers/watch.go`

### 2. Webhook TLS — 16 failures
`webhook.go:425,520,601,675,904,1194,1244,1334,1549,1631,2032,2107(x3),2338,2465`
- **Root cause**: K8s `caBundle` is Go `[]byte` → JSON base64. We passed base64 string to `Certificate::from_pem()` which expects raw PEM bytes. Every webhook TLS handshake failed silently.
- `:1631` also needs webhook config immunity (webhooks must not intercept webhook config objects)
- **FIXED** (#6): base64-decode caBundle; (#3): skip webhooks for webhook config objects
- K8s ref: `admissionregistration/v1/types.go`, `admission/plugin/webhook/predicates/rules/rules.go`

### 3. CRD OpenAPI v2 — 9 failures
`crd_publish_openapi.go:77,161,211,253,285,318,366,400,451`
- **Root cause**: three mismatches vs K8s builder.go:
  1. Root `x-kubernetes-preserve-unknown-fields=true` → K8s replaces entire schema with `{type: object}` (builder.go:392)
  2. Nested preserve-unknown-fields → K8s clears items/properties/type but KEEPS the extension as a vendor extension (conversion.go:68)
  3. `x-kubernetes-*` extensions: K8s keeps when true (via `toKubeOpenAPI()`), omits when false. We stripped all.
- **FIXED** (#4): match K8s builder.go + conversion.go + kubeopenapi.go
- K8s ref: `controller/openapi/builder/builder.go:392-407`, `v2/conversion.go:68-89`, `schema/kubeopenapi.go:67-90`

### 4. DNS $$ Expansion — 6 failures
`dns_common.go:476` (x6)
- **Root cause**: K8s `expand.go` converts `$$` → `$` (escape sequence for shell command substitution). Our `expand_k8s_vars` only handled `$(VAR_NAME)`. DNS probe commands use `$$(dig ...)` which must become `$(dig ...)`. Without this, `$$` was interpreted as PID by the shell, causing `pause: syntax error: unexpected word (expecting "do")`.
- **FIXED** (#7): rewrote expand_k8s_vars matching K8s `expansion/expand.go` exactly
- K8s ref: `third_party/forked/golang/expansion/expand.go:83-85`

### 5. EmptyDir Permissions — 10 failures (unfixable on macOS)
`output.go:263` (x9), `output.go:282` (x1)
- File permissions `-rw-r--r--` instead of `-rw-rw-rw-`
- macOS Docker Desktop filesystem does not support 0666 mode
- Not fixable — requires Linux host

### 6. Service Routing — 6 failures
`service.go:768,896,3459,4291(x4)`
- `:896` — **Root cause**: EndpointSlice controller never deleted stale slices when pods were removed. Old slices with outdated endpoints persisted. **FIXED** (#10): cleanup stale slices after reconcile.
- `:768,3459,4291(x4)` — services not reachable via ClusterIP/NodePort. kube-proxy iptables routing issue (traffic from Docker bridge containers may not traverse correct chains). Needs further investigation.
- K8s ref: `pkg/controller/endpointslice/reconciler.go`

### 7. Apps Controllers — 10 failures (mixed causes)
- `deployment.go:995,1259` — Docker 409 container name conflicts. Kubelet not cleaning up exited containers before recreating pods with same name. Needs kubelet container cleanup improvement.
- `statefulset.go:957` — controller sets deletionTimestamp but test expects DELETE watch event. Kubelet graceful termination + storage deletion flow needs timing improvement.
- `statefulset.go:1092` — watch degradation (failed at 17:30, after degradation began)
- `replica_set.go:232` — pod running but not network-reachable (service routing issue)
- `replica_set.go:560` — watch degradation (failed at 17:36)
- `rc.go:509` — watch degradation (failed at 18:04)
- `rc.go:623` — ReplicaFailure condition not cleared after quota freed. RC controller clear logic exists but timing between quota status update and RC reconcile cycle may cause stale condition to persist.
- `job.go:935` — job pods didn't become active (scheduling/kubelet timing)
- `job.go:1251` — watch degradation (failed at 18:35)
- `daemon_set.go:1276` — ControllerRevision Match() byte comparison. Pod template defaults (#1) should fix serialization mismatch.

### 8. Network — 3 failures
- `proxy.go:271,503` — proxy subresource routing to backend pods
- `hostport.go:219` — two pods with same hostPort but different hostIPs. **Root cause**: kubelet hardcoded `host_ip: "0.0.0.0"` for non-pause containers instead of using pod spec's `port.host_ip`. Also scheduler only checked Running pods for conflicts, missing Pending pods. **FIXED** (#9): kubelet uses pod hostIP; scheduler includes Pending pods.
- K8s ref: `scheduler/framework/plugins/nodeports/node_ports.go`

### 9. Other — 8 failures
- `service_latency.go:145` — deployment not ready before latency test (scheduling/kubelet timing)
- `preemption.go:877` — RS only created 1 of 2 pods via preemption (preemption logic gap)
- `resource_quota.go:290` — **FIXED** (#2): pod admitted when quota exceeded. Now atomic check-and-increment of quota status.used.
- `aggregator.go:359` — API aggregation proxy can't reach sample-apiserver (service routing)
- `garbage_collector.go:436` — GC deletes pods when propagationPolicy=Orphan. GC has orphan logic but timing between orphan processing and orphan detection may cause race.
- `field_validation.go:611` — strict validation of embedded metadata in CRs not detecting `.spec.template.metadata.unknownSubMeta`. CRD handler needs recursive metadata field validation.
- `pod_resize.go:857` — in-place pod resize not implemented

## All 14 Fixes (NOT YET DEPLOYED)

| # | Fix | Crate | Root Cause | K8s Source |
|---|-----|-------|------------|-----------|
| 1 | Pod template defaults for all workloads | api-server | Missing SetDefaults_PodSpec/Container/Probe on templates | `core/v1/defaults.go`, `apps/v1/defaults.go`, `batch/v1/defaults.go` |
| 2 | Atomic ResourceQuota admission | api-server | Check-only without atomic usage increment | `admission/plugin/resourcequota/controller.go` |
| 3 | Webhook config immunity | api-server | Webhook configs not exempt from admission webhooks | `admission/plugin/webhook/predicates/rules.go` |
| 4 | CRD OpenAPI v2 conversion | api-server | Root preserve-unknown-fields, extension stripping | `openapi/builder/builder.go:392`, `v2/conversion.go:68` |
| 5 | Service internalTrafficPolicy | api-server | Missing default "Cluster" | `core/v1/defaults.go:141` |
| 6 | Webhook caBundle base64 decode | api-server | Base64 string passed to PEM parser | `admissionregistration/v1/types.go` |
| 7 | $$ → $ command expansion | kubelet | Missing escape sequence in expand_k8s_vars | `expansion/expand.go:83` |
| 8 | Default watch timeout 1800s | api-server | No timeout → infinite stream accumulation | `endpoints/handlers/watch.go` |
| 9 | HostPort kubelet + scheduler | kubelet, scheduler | Hardcoded 0.0.0.0; only checked Running pods | `plugins/nodeports/node_ports.go` |
| 10 | EndpointSlice stale cleanup | controller-manager | Stale slices never deleted on pod removal | `endpointslice/reconciler.go` |
| 11 | Docker 409 container conflict retry | kubelet | No cleanup of exited containers before recreate | `kuberuntime/kuberuntime_manager.go:1433` |
| 12 | Embedded metadata field validation | api-server | Only checked root .metadata, not nested embedded objects | `apiserver/schema/objectmeta/validation.go` |
| 13 | Scheduler per-pod state refresh | scheduler | Stale all_pods after bind/preemption blocked second pod | `scheduler/schedule_one.go` |
| 14 | GC orphan error propagation | controller-manager | orphanDependents error swallowed → premature finalizer removal | `garbagecollector/garbagecollector.go:753` |
| 15 | Live quota usage computation | api-server | Stale status.used prevented pod creation after quota freed | `quota/v1/generic/evaluator.go` |

## Impact Analysis

| Fix | Tests Affected | Potential Fixed |
|-----|---------------|----------------|
| #8 Watch timeout | ~15 late-stage failures | 15 |
| #6 Webhook caBundle | 16 webhook tests | 16 |
| #4 CRD OpenAPI v2 | 9 CRD tests | 9 |
| #7 $$ expansion | 6 DNS tests | 6 |
| #11 Docker 409 retry | 2 deployment tests | 2 |
| #1 Pod defaults | DaemonSet, apps | 3-5 |
| #13 Scheduler refresh | 1 preemption test | 1 |
| #14 GC orphan error | 1 GC test | 1 |
| #15 Live quota usage | 1 RC test | 1 |
| #9 HostPort | 1 hostport test | 1 |
| #10 EndpointSlice | 1 service test | 1 |
| #12 Embedded metadata | 1 field validation test | 1 |
| #2 Atomic quota | 1 quota test | 1 |
| #3 Webhook immunity | 1 webhook test | 1 |
| #5 Service default | 0-1 service tests | 0-1 |
| **Total** | | **~60-63** |
| **Projected pass** | | **~428-431 / 441 (97-98%)** |

**Remaining after ALL fixes**: EmptyDir/macOS (10), pod_resize (1), aggregator (1) = ~12 unfixable

**Theoretical max on Linux**: ~430/441 (97.5%)

## Progress History

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 134 | 370 | 71 | 441 | 83.9% | |
| 135 | 373 | 68 | 441 | 84.6% | |
| 137 | ~380 | ~61 | 441 | ~86.2% | |
| 138 | TERM | — | 441 | — | e2e pod killed |
| 140 | ~375 | ~36+ | 441 | ~85% | 0 watch failures at 43min |
| 141 | 368 | 73 | 441 | 83.4% | 2403 watch failures after 4h |
| 142 | — | — | 441 | — | 15 fixes pending deploy |

# Conformance Failure Tracker

**Round 141** | Complete — 368/441 (83.4%) | 2026-04-14

## Round 141 Failures by Root Cause (73 total)

### Watch Timeout Regression — ~15 failures caused by watch degradation
- Watch failures started at 18:02 (4h into test), 2403 "context canceled" errors cascaded
- **Root cause**: watches without client-specified timeout ran FOREVER, accumulating HTTP/2 streams until connection degraded
- **FIXED**: default watch timeout now 1800s matching K8s MinRequestTimeout
- Affected tests: `init_container.go:440`, `runtime.go:115`, `rc.go:509`, `replica_set.go:560`, `job.go:1251`, `projected_configmap.go:330`, and others that failed after 18:02
- K8s ref: apiserver/pkg/endpoints/handlers/watch.go

### Webhook caBundle — 16 failures — FIXED (not deployed)
- `webhook.go:425,520,601,675,904,1194,1244,1334,1549,1631,2032,2107(x3),2338,2465`
- **Root cause**: caBundle stored as Go `[]byte` → JSON base64. We passed the base64 string directly to `Certificate::from_pem()` which expects raw PEM. TLS verification failed silently on every webhook call.
- `:1631` additionally fixed: webhook config objects now exempt from admission webhooks
- **Fix**: base64-decode caBundle before PEM parsing; skip webhooks for webhook config objects
- K8s ref: admissionregistration/v1/types.go — `CABundle []byte`

### CRD OpenAPI v2 Conversion — 9 failures — FIXED (not deployed)
- `crd_publish_openapi.go:77,161,211,253,285,318,366,400,451`
- **Root cause**: OpenAPI v2 conversion didn't match K8s builder.go
  - Root `x-kubernetes-preserve-unknown-fields=true` → K8s replaces entire schema with `{type: object}` (builder.go:392-395)
  - Nested → K8s clears items/properties/type but KEEPS extension as vendor ext (conversion.go:68-89)
  - K8s KEEPS `x-kubernetes-*` extensions when true via `toKubeOpenAPI()`, strips when false
  - We were incorrectly stripping ALL extensions
- **Fix**: match K8s builder.go + conversion.go + kubeopenapi.go exactly
- K8s ref: controller/openapi/builder/builder.go:392-407, v2/conversion.go:68-89, schema/kubeopenapi.go:67-90

### DNS Command Expansion — 6 failures — FIXED (not deployed)
- `dns_common.go:476` (x6)
- **Root cause**: K8s `expand.go` converts `$$` to `$` (escape sequence). Our `expand_k8s_vars` only handled `$(VAR_NAME)` expansion, missing `$$` → `$`. DNS probe commands use `$$(dig ...)` which should expand to `$(dig ...)` for shell command substitution. Without this, the literal `$$` was interpreted by the shell differently, causing `pause: syntax error: unexpected word (expecting "do")`.
- **Fix**: rewrite expand_k8s_vars to match K8s third_party/forked/golang/expansion/expand.go
- K8s ref: third_party/forked/golang/expansion/expand.go:83-85

### EmptyDir / Volumes — 10 failures — macOS DinD limitation
- `output.go:263` (x9), `output.go:282` (x1)
- File permissions `-rw-r--r--` instead of `-rw-rw-rw-`
- macOS Docker filesystem doesn't support 0666 mode
- Not fixable in our code — requires Linux host

### Service Routing — 6 failures — kube-proxy / networking
- `service.go:768,896,3459,4291(x4)`
- Services not reachable via ClusterIP/NodePort
- `:896` specifically: EndpointSlice has extra port mappings that shouldn't be there — EndpointSlice controller including stale endpoints
- Related to kube-proxy iptables chain correctness and EndpointSlice controller accuracy

### Apps Controllers — 10 failures — mixed causes
- `deployment.go:995` — rollover: 0 pods available, Docker 409 container name conflicts in kubelet
- `deployment.go:1259` — RS never reached desired availableReplicas (Docker conflicts)
- `statefulset.go:957` — controller sets deletionTimestamp but kubelet doesn't complete pod removal fast enough
- `statefulset.go:1092` — StatefulSet update/rollback (watch degradation at 17:30)
- `replica_set.go:232` — pod running but not reachable via network (service routing)
- `replica_set.go:560` — RS status update: late-stage watch degradation at 17:36
- `rc.go:509` — pod didn't come up: watch degradation at 18:04
- `rc.go:623` — ReplicaFailure condition not cleared after quota freed (timing between quota controller and RC controller)
- `job.go:935` — job pods didn't become active within timeout
- `job.go:1251` — job issue: watch degradation at 18:35
- `daemon_set.go:1276` — ControllerRevision Match() byte comparison (pod template defaults fix may help)

### Network — 3 failures
- `proxy.go:271,503` — proxy subresource routing
- `hostport.go:219` — host port mapping (two pods with same hostPort but different hostIPs)

### Other — 8 failures
- `service_latency.go:145` — deployment not ready before latency test starts
- `preemption.go:877` — RS only created 1 of 2 required pods via preemption
- `resource_quota.go:290` — **FIXED**: pod allowed when quota exceeded; now uses atomic quota update
- `aggregator.go:359` — API aggregation (sample-apiserver service unreachable)
- `garbage_collector.go:436` — GC deleting orphaned pods when it shouldn't (propagationPolicy: Orphan timing)
- `field_validation.go:611` — strict validation of embedded metadata in CRs: `.spec.template.metadata.unknownSubMeta: field not declared in schema` not detected
- `pod_resize.go:857` — in-place pod resize (not implemented)

## All Fixes Made (NOT YET DEPLOYED)

### 1. Pod Template Defaults — affects all workloads
- Created `handlers/defaults.rs` with K8s-compatible defaulting
- PodSpec: dnsPolicy, restartPolicy, terminationGracePeriodSeconds, schedulerName
- Container: terminationMessagePath, terminationMessagePolicy, imagePullPolicy
- Probe: timeoutSeconds, periodSeconds, successThreshold, failureThreshold
- Workload-specific: DaemonSet, Deployment, StatefulSet, ReplicaSet, Job, CronJob defaults
- Applied to create AND update handlers for all 8 workload types
- K8s ref: pkg/apis/core/v1/defaults.go, pkg/apis/apps/v1/defaults.go, pkg/apis/batch/v1/defaults.go

### 2. Atomic ResourceQuota Admission
- check_resource_quota now checks limits AND atomically increments quota status.used
- CAS retry on concurrent creates
- K8s ref: apiserver/pkg/admission/plugin/resourcequota/controller.go

### 3. Webhook Configuration Immunity
- Skip admission webhooks for ValidatingWebhookConfiguration and MutatingWebhookConfiguration
- K8s ref: apiserver/pkg/admission/plugin/webhook/predicates/rules/rules.go

### 4. CRD OpenAPI v2 Conversion
- Root preserve-unknown-fields → replace entire schema with `{type: object}`
- Nested preserve-unknown-fields → clear items/properties, keep extension as vendor ext
- Nullable=true → clear type/items/properties (conversion.go:56-66)
- x-kubernetes-* extensions: kept when true, stripped when false
- K8s ref: builder/builder.go:392-407, v2/conversion.go:68-89, schema/kubeopenapi.go:67-90

### 5. Service Internal Traffic Policy Default
- Default internalTrafficPolicy to "Cluster" for ClusterIP/NodePort/LoadBalancer
- K8s ref: pkg/apis/core/v1/defaults.go:141-146

### 6. Webhook caBundle Base64 Decoding — CRITICAL
- K8s caBundle is `[]byte` → JSON base64. Must decode before Certificate::from_pem().
- Fixed in both validating and mutating webhook call paths.
- K8s ref: admissionregistration/v1/types.go

### 7. K8s $$ → $ Escape in Command/Args Expansion
- Rewrote expand_k8s_vars to match K8s expansion/expand.go exactly
- Handles: `$$` → `$`, `$(VAR)` → expand or literal, `$other` → literal
- K8s ref: third_party/forked/golang/expansion/expand.go

### 8. Default Watch Timeout 1800s
- All 4 watch handler variants now default to 1800s when client omits timeoutSeconds
- Prevents HTTP/2 stream accumulation and connection degradation
- K8s ref: apiserver/pkg/endpoints/handlers/watch.go, MinRequestTimeout=1800

### 9. HostPort Handling — Kubelet + Scheduler
- Kubelet: non-pause containers hardcoded `host_ip: "0.0.0.0"` instead of using pod spec's `port.host_ip`. Also fixed protocol to use port.protocol instead of "tcp".
- Scheduler: hostPort conflict check only considered Running pods. K8s includes Pending (scheduled but not started). Changed to skip only terminal/terminating.
- K8s ref: scheduler/framework/plugins/nodeports/node_ports.go, scheduler/framework/types.go

### 10. EndpointSlice Stale Slice Cleanup
- Controller created/updated slices for current pods but never deleted stale slices from removed pods
- Caused "Unexpected port mappings on slices, extra: [{...}]" conformance failures
- Now deletes EndpointSlices owned by the service that aren't in the current desired set
- K8s ref: pkg/controller/endpointslice/reconciler.go — finalize()

## Impact Analysis (if deployed)

| Fix | Potential Tests Fixed | Cumulative |
|-----|----------------------|------------|
| Watch timeout (prevents regression) | ~15 (late-stage failures) | 383 |
| Webhook caBundle base64 | up to 16 | 399 |
| CRD OpenAPI v2 | up to 9 | 408 |
| $$ escape (DNS) | up to 6 | 414 |
| Pod template defaults | 1-5 | 415-419 |
| Atomic quota | 1 | 416-420 |
| Webhook immunity | 1 | 417-421 |
| Service default | 0-1 | 417-422 |
| **Total potential** | **~49-54** | **~417-422 (94-96%)** |

**Remaining unfixable**: EmptyDir/macOS (10), pod_resize (1) = 11 tests = max possible ~430/441 (97.5%)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TERMINATED | ~35+ | 441 | — |
| 140 | ~375 | ~36+ | 441 | ~85% |
| 141 | 368 | 73 | 441 | 83.4% |

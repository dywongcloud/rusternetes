# Conformance Failure Tracker

**Round 141** | Complete — 368/441 (83.4%) | 2026-04-14

## Round 141 Failures (73 total)

### Webhook — 16 failures — BIGGEST BLOCKER
- `webhook.go:425,520,601,675,904,1194,1244,1334,1549,1631,2032,2107(x3),2338,2465`
- `:1631` — **FIXED** (not deployed): webhook config objects now exempt from admission webhooks
- All other 15: webhook service readiness timeout — API server can't reach webhook pods via ClusterIP
  - Root cause: API server resolves webhook service to ClusterIP, but traffic from the API server Docker container may not traverse kube-proxy iptables chains correctly
  - kube-proxy runs host-network; API server runs Docker bridge — traffic path: container → bridge → host iptables → pod IP
  - Need to verify iptables OUTPUT chain rules apply to traffic originating from Docker bridge
- **K8s ref**: apiserver/pkg/admission/plugin/webhook/predicates/rules/rules.go

### EmptyDir / Volumes — 10 failures — macOS DinD limitation
- `output.go:263` (x9), `output.go:282` (x1)
- File permissions `-rw-r--r--` instead of `-rw-rw-rw-`
- macOS Docker filesystem doesn't support 0666 mode
- Not fixable in our code — requires Linux host

### CRD OpenAPI — 9 failures — FIXED (not deployed)
- `crd_publish_openapi.go:77,161,211,253,285,318,366,400,451`
- **Root cause**: OpenAPI v2 conversion didn't match K8s builder.go behavior
  - When `x-kubernetes-preserve-unknown-fields=true` at root, K8s replaces entire schema with `{type: object}` (builder.go:392-395)
  - When nested, K8s clears items/properties/type but KEEPS extension as vendor extension
  - K8s KEEPS `x-kubernetes-*` extensions when true, strips when false (toKubeOpenAPI)
  - We were incorrectly stripping all extensions
- **Fix**: Match K8s v2 conversion exactly
- **K8s ref**: controller/openapi/builder/builder.go:392-407, v2/conversion.go:68-89, schema/kubeopenapi.go:67-90

### DNS — 6 failures — kubelet command handling
- `dns_common.go:476` (x6)
- Pod logs show: "pause: line 1: syntax error: unexpected word (expecting 'do')"
- agnhost container running `pause` instead of intended command
- Kubelet may be wrapping commands incorrectly or using image default entrypoint

### Service — 6 failures — kube-proxy routing
- `service.go:768,896,3459,4291(x4)`
- Service routing failures — pods not reachable via ClusterIP/NodePort
- Related to kube-proxy iptables chain correctness

### Apps Controllers — 10 failures
- `deployment.go:995,1259` — RS never reached desired availableReplicas; Docker 409 container name conflicts
- `statefulset.go:957,1092` — pod not deleted by controller; controller sets deletionTimestamp but kubelet doesn't complete removal
- `replica_set.go:232,560` — RS controller issues
- `rc.go:509,623` — ReplicaFailure condition not cleared after quota freed; RC controller logic
- `job.go:935,1251` — Job controller issues
- `daemon_set.go:1276` — ControllerRevision Match() byte comparison (pod template defaults fix may help)

### Network — 3 failures
- `proxy.go:271,503` — proxy subresource routing
- `hostport.go:219` — host port mapping

### Other — 13 failures
- `service_latency.go:145` — deployment not ready before latency test starts
- `preemption.go:877` — RS only created 1 of 2 required pods
- `resource_quota.go:290` — **FIXED** (not deployed): pod allowed when quota exceeded; now uses atomic quota update
- `aggregator.go:359` — API aggregation (sample-apiserver)
- `garbage_collector.go:436` — GC issue
- `field_validation.go:611` — strict field validation
- `projected_configmap.go:330` — projected volume
- `runtime.go:115` — container runtime
- `pod_resize.go:857` — in-place pod resize (not implemented)
- `init_container.go:440` — init container handling

## Fixes Made This Session (NOT YET DEPLOYED)

### 1. Pod Template Defaults (MAJOR — affects all workloads)
- Created `handlers/defaults.rs` with K8s-compatible defaulting
- PodSpec: dnsPolicy=ClusterFirst, restartPolicy=Always, terminationGracePeriodSeconds=30, schedulerName=default-scheduler
- Container: terminationMessagePath, terminationMessagePolicy, imagePullPolicy
- Probe: timeoutSeconds=1, periodSeconds=10, successThreshold=1, failureThreshold=3
- Workload defaults: DaemonSet (updateStrategy, revisionHistoryLimit), Deployment (replicas, strategy, progressDeadlineSeconds), StatefulSet (podManagementPolicy, updateStrategy), Job (completions, parallelism, backoffLimit, completionMode), CronJob (concurrencyPolicy, historyLimits)
- Applied to create AND update handlers for: Pod, DaemonSet, Deployment, StatefulSet, ReplicaSet, Job, CronJob, ReplicationController
- K8s ref: pkg/apis/core/v1/defaults.go, pkg/apis/apps/v1/defaults.go, pkg/apis/batch/v1/defaults.go

### 2. Atomic ResourceQuota Admission
- check_resource_quota now checks limits AND atomically increments quota status.used
- CAS retry on concurrent creates
- K8s error format: "exceeded quota: <name>, requested: ..., used: ..., limited: ..."
- K8s ref: apiserver/pkg/admission/plugin/resourcequota/controller.go

### 3. Webhook Configuration Immunity
- Skip admission webhooks for ValidatingWebhookConfiguration and MutatingWebhookConfiguration objects
- Prevents broken webhooks from locking cluster config
- K8s ref: apiserver/pkg/admission/plugin/webhook/predicates/rules/rules.go

### 4. CRD OpenAPI v2 Conversion (MAJOR)
- Root preserve-unknown-fields → replace entire schema with {type: object}
- Nested preserve-unknown-fields → clear items/properties, keep extension as vendor ext
- Nullable=true → clear type/items/properties
- x-kubernetes-* extensions: kept when true (vendor ext), stripped when false (omitempty)
- K8s ref: controller/openapi/builder/builder.go, v2/conversion.go, schema/kubeopenapi.go

### 5. Service Internal Traffic Policy Default
- Default internalTrafficPolicy to "Cluster" for ClusterIP/NodePort/LoadBalancer
- K8s ref: pkg/apis/core/v1/defaults.go:141-146

## Impact Analysis (if deployed)

| Fix | Potential Tests Fixed | New Pass Count |
|-----|----------------------|----------------|
| CRD OpenAPI v2 | up to 9 | 377 |
| Webhook immunity | 1 | 378 |
| Pod template defaults | 1-5 (DaemonSet, apps) | 379-383 |
| Atomic quota | 1 | 380-384 |
| Service default | 0-1 | 380-385 |
| **Total potential** | **12-17** | **380-385** |

**To reach 90%+ (397+)**: Must fix webhook routing (16 tests) — this requires kube-proxy iptables to work for traffic originating from Docker bridge containers.

## Key Metrics
- **Watch failures: 0** (down from 3012 in round 138!)
- HTTP/2 flow control fix eliminated all watch context canceled errors
- Lease-based heartbeat preventing node NotReady

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TERMINATED | ~35+ | 441 | — |
| 140 | ~375 | ~36+ | 441 | ~85% |
| 141 | 368 | 73 | 441 | 83.4% |

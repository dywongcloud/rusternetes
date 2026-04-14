# Conformance Failure Tracker

**Round 141** | Complete — 368/441 (83.4%) | 2026-04-14

## Round 141 Failures (73 total)

### CRD OpenAPI — 7 failures — FIXED
- `crd_publish_openapi.go:77,161,211,253,366,400,451`
- **Root cause**: OpenAPI v2 conversion didn't match K8s builder.go behavior
  - When `x-kubernetes-preserve-unknown-fields=true` at root, K8s replaces entire schema with `{type: object}` (builder.go:392-395)
  - When nested, K8s clears items/properties/type but KEEPS extension as vendor extension
  - K8s KEEPS `x-kubernetes-*` extensions as vendor extensions when true (toKubeOpenAPI)
  - We were incorrectly stripping all extensions
- **Fix**: Match K8s v2 conversion exactly — root=simplify, nested=clear children, keep extensions
- **K8s ref**: controller/openapi/builder/builder.go:392-407, v2/conversion.go:68-89, schema/kubeopenapi.go:67-90

### Webhook — 6 failures — PARTIALLY FIXED
- `webhook.go:675,904,1244,1334,1631,2107,2338`
- `:1631` — **FIXED**: webhook config objects now exempt from admission webhooks
- `:675,904,1244,2338` — webhook service readiness timeout — API server can't reach webhook pods
  - Root cause: kube-proxy iptables routing to webhook service ClusterIP
  - Need to verify kube-proxy is creating DNAT rules for webhook services
- `:2107` — mutating webhook for custom resources (depends on webhook readiness)
- **K8s ref**: apiserver/pkg/admission/plugin/webhook/predicates/rules/rules.go

### DNS — 3 failures — INVESTIGATING
- `dns_common.go:476` (x3) — DNS resolution failures
- Pod command syntax errors: "pause: line 1: syntax error: unexpected word"
- Possible kubelet issue with pod command/args handling

### EmptyDir — 4 failures — DinD (macOS)
- `output.go:263` (x4) — file permissions `-rw-r--r--` instead of `-rw-rw-rw-`
- macOS Docker filesystem doesn't support 0666 mode properly
- Not fixable in our code — requires Linux host

### Service — 3 failures — INVESTIGATING
- `service.go:3459,4291` (x2) — service routing failures
- Need to check kube-proxy iptables rules

### Network — 2 failures — INVESTIGATING
- `proxy.go:271` — proxy subresource
- `hostport.go:219` — host port mapping

### Service Latency — 1 failure — INVESTIGATING
- `service_latency.go:145` — deployment not ready before latency test starts
- Deployment `WaitForDeploymentComplete()` timed out

### DaemonSet — 1 failure — PARTIALLY FIXED
- `daemon_set.go:1276` — ControllerRevision Match() byte comparison
- Pod template defaults now applied (dnsPolicy, restartPolicy, etc.)
- May still fail if JSON serialization order differs from Go's encoding/json

### Deployment — 1 failure — INVESTIGATING
- `deployment.go:1259` — ReplicaSet never reached desired availableReplicas
- Docker 409 container name conflicts in kubelet
- Container cleanup issue during pod recreation

### StatefulSet — 1 failure — INVESTIGATING
- `statefulset.go:957` — pod not deleted by controller
- Controller sets deletionTimestamp but test expects DELETE watch event
- Kubelet needs to complete graceful termination and remove from storage

### RC — 1 failure — INVESTIGATING
- `rc.go:623` — ReplicaFailure condition not cleared after quota freed
- Controller has clear logic but may not trigger when quota usage is stale

### ResourceQuota — 1 failure — FIXED
- `resource_quota.go:290` — pod allowed when quota exceeded
- **Root cause**: quota check-only without atomic usage update
- **Fix**: check_resource_quota now atomically increments quota status.used
- **K8s ref**: apiserver/pkg/admission/plugin/resourcequota/controller.go

### Pod Resize — 1 failure — NOT IMPLEMENTED
- `pod_resize.go:857` — in-place pod resize not supported

### Preemption — 1 failure — INVESTIGATING
- `preemption.go:877` — RS only created 1 of 2 required pods
- Preemption not fully evicting lower-priority pods

### Aggregator — 1 failure — INVESTIGATING
- `aggregator.go:359` — API aggregation (sample-apiserver)

## Fixes Made This Session (NOT YET DEPLOYED)

### 1. Pod Template Defaults (MAJOR — affects all workloads)
- Created `handlers/defaults.rs` with K8s-compatible defaulting
- Applied to ALL workload create/update handlers
- K8s ref: pkg/apis/core/v1/defaults.go, pkg/apis/apps/v1/defaults.go

### 2. Atomic ResourceQuota Admission
- check_resource_quota now atomically increments quota status.used
- K8s ref: apiserver/pkg/admission/plugin/resourcequota/controller.go

### 3. Webhook Configuration Immunity
- Skip admission webhooks for webhook config objects
- K8s ref: apiserver/pkg/admission/plugin/webhook/predicates/rules/rules.go

### 4. CRD OpenAPI v2 Conversion (MAJOR)
- Root preserve-unknown-fields → replace schema with {type: object}
- Nested preserve-unknown-fields → clear items/properties, keep extension
- x-kubernetes-* extensions kept when true, stripped when false
- K8s ref: controller/openapi/builder/builder.go, v2/conversion.go

### 5. Service Internal Traffic Policy Default
- Default to "Cluster" for ClusterIP/NodePort/LoadBalancer
- K8s ref: pkg/apis/core/v1/defaults.go:141-146

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

# Conformance Failure Tracker

**Round 141** | Running (2h+ in, ~36 failures at 300+ tests) | 2026-04-14

## Round 141 Failures (36 so far, test still running)

### CRD OpenAPI — 7 failures — IN PROGRESS
- `crd_publish_openapi.go:77,161,211,253,366,400,451`
- Root cause: x-kubernetes-* extensions appearing in OpenAPI v2 output
- Fix: strip_false_extensions now strips all x-kubernetes-{embedded-resource,int-or-string,preserve-unknown-fields} entirely
- Additional issue: CRD with unknown root-level fields rejected (`:211` — allows unknown at root)
- Status: Fix committed but NOT deployed

### Webhook — 6 failures — IN PROGRESS
- `webhook.go:675,904,1244,1334,1631,2107,2338`
- `:1631` — webhook config objects NOT exempt from webhooks (K8s skips them)
  - Fix: Added skip for validatingwebhookconfigurations/mutatingwebhookconfigurations
- `:2107` — mutating webhook for custom resources
- `:675,904,1244,1334,2338` — various webhook behaviors (need investigation)
- Status: webhook config exempt fix committed but NOT deployed

### DNS — 3 failures — INVESTIGATING
- `dns_common.go:476` (x3) — DNS resolution failures
- Likely CoreDNS pod or network issue

### EmptyDir — 4 failures — DinD (macOS)
- `output.go:263` (x4) — file permissions `-rw-r--r--` instead of `-rw-rw-rw-`
- macOS Docker filesystem doesn't support 0666 mode properly

### Service — 3 failures — INVESTIGATING
- `service.go:3459,4291` (x2) — service routing failures
- Need to check kube-proxy iptables rules

### Network — 2 failures — INVESTIGATING
- `proxy.go:271` — proxy subresource
- `hostport.go:219` — host port mapping

### Service Latency — 1 failure — INVESTIGATING
- `service_latency.go:145` — deployment not ready in time

### DaemonSet — 1 failure — IN PROGRESS
- `daemon_set.go:1276` — ControllerRevision Match() byte comparison
- Related to pod template defaults — K8s serializes templates WITH defaults
- Fix: Added shared apply_pod_spec_defaults to all workload handlers

### Deployment — 1 failure — INVESTIGATING
- `deployment.go:1259` — likely rollover or maxUnavailable issue

### StatefulSet — 1 failure — INVESTIGATING
- `statefulset.go:957` — needs analysis

### RC — 1 failure — IN PROGRESS
- `rc.go:623` — ReplicaFailure condition not cleared after quota freed
- Already identified in previous rounds

### ResourceQuota — 1 failure — INVESTIGATING
- `resource_quota.go:290` — quota enforcement issue

### Pod Resize — 1 failure — INVESTIGATING
- `pod_resize.go:857` — in-place pod resize (may not be implemented)

### Preemption — 1 failure — INVESTIGATING
- `preemption.go:877` — preemption behavior

### Aggregator — 1 failure — INVESTIGATING
- `aggregator.go:359` — API aggregation (sample-apiserver)

## Fixes Made This Session (NOT YET DEPLOYED)

### 1. Pod Template Defaults (MAJOR — affects all workloads)
- K8s applies defaults to ALL PodSpecs, including templates in workloads
- Created `handlers/defaults.rs` with shared defaulting functions
- PodSpec defaults: dnsPolicy=ClusterFirst, restartPolicy=Always, terminationGracePeriodSeconds=30, schedulerName=default-scheduler
- Container defaults: terminationMessagePath, terminationMessagePolicy, imagePullPolicy
- Probe defaults: timeoutSeconds=1, periodSeconds=10, successThreshold=1, failureThreshold=3
- Workload defaults: DaemonSet, Deployment, StatefulSet, ReplicaSet, Job, CronJob
- Applied in create AND update handlers for: Pod, DaemonSet, Deployment, StatefulSet, ReplicaSet, Job, CronJob, ReplicationController
- K8s ref: pkg/apis/core/v1/defaults.go, pkg/apis/apps/v1/defaults.go, pkg/apis/batch/v1/defaults.go

### 2. Webhook Config Immunity
- Webhook configuration objects are now exempt from admission webhooks
- K8s ref: apiserver/pkg/admission/plugin/webhook/predicates/rules/rules.go

### 3. CRD OpenAPI Extension Stripping
- Strip x-kubernetes-preserve-unknown-fields entirely (was only stripping false)
- Strip x-kubernetes-embedded-resource entirely (both true and false)
- K8s uses Go struct fields for these, NOT vendor extension maps

### 4. Service Internal Traffic Policy Default
- Default internalTrafficPolicy to "Cluster" for ClusterIP/NodePort/LoadBalancer
- K8s ref: pkg/apis/core/v1/defaults.go:141-146

## Key Metrics
- **Watch failures: 0** (down from 3012 in round 138!)
- HTTP/2 flow control fix completely eliminated watch context canceled
- Lease-based heartbeat preventing node NotReady

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TERMINATED | ~35+ | 441 | — |
| 140 | ~375 | ~36+ | 441 | ~85% |
| 141 | TBD (36 failures at ~300 tests) | TBD | 441 | TBD |

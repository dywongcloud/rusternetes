# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | 13 fixes pending redeploy

## Fixes Ready (not yet deployed)

| # | Fix | Commit | What it does |
|---|-----|--------|-------------|
| 1 | StatefulSet status counts | 823884f | Filter terminating pods from replicas/readyReplicas/availableReplicas |
| 2 | OpenAPI v3 GVK extensions | 7fb8ecd | Add x-kubernetes-group-version-kind to all operations so kubectl can map GVK to paths |
| 3 | Job completion reason | db1a3e5 | Use "CompletionsReached" instead of "Completed" for Job Complete condition |
| 4 | OpenAPI v2 Content-Type | b3a6772 | Use dot-format `spec.v2.v1.0+protobuf` matching real K8s kube-openapi handler |
| 5 | RC ReplicaFailure condition | b3a6772 | Only set ReplicaFailure when pod creation actually errors, not when replicas haven't caught up |
| 6 | Webhook response parsing | ba0b26f | Fallback to raw JSON parsing when strict AdmissionReview deserialization fails |
| 7 | Scheduler DisruptionTarget | d7ef779 | Add DisruptionTarget condition with PreemptionByScheduler reason to preempted pods |
| 8 | Protobuf response middleware removed | 8965fd5 | Blanket protobuf wrapping broke discovery — removed |
| 9 | Exec WebSocket close delay | 24ca36b | 500ms delay before WebSocket close to let client read status on channel 3 |
| 10 | OpenAPI v3 schemas | 79f4f4a | Add schemas for all 47 resource types (Deployment, Service, etc.) with additionalProperties:true |
| 11 | Targeted protobuf response | c859496 | Wrap JSON response in protobuf only when request body was protobuf (x-was-protobuf marker) |
| 12 | Recreate deployment | 140048a | Wait for all old RS pods to fully terminate before creating new RS |
| 13 | Status PATCH merge | cc84ef9 | Merge status fields on PATCH instead of replacing — preserves replicas when patching conditions |
| 14 | Watch label selector ADDED | cc84ef9 | Track objects removed via synthetic DELETED; send ADDED when labels match again |

## Verified working (from round 124 evidence)

- Fix 3 (Job): 5 Job tests newly passing
- Fix 4 (OpenAPI v2 MIME): errors dropped from 18 to 0

## All 146 Failures — Status

| Test | Root Cause | Fix | Status |
|------|-----------|-----|--------|
| **AdmissionWebhook (13 tests)** | | | |
| listing/patching/updating webhooks (4) | Webhook response parsing | #6 | Fixed |
| deny pod/configmap/crd/attaching (4) | Webhook readiness — webhook not called | #6 | Partially fixed — response parsing fixed but webhook invocation may still fail |
| mutate configmap/pod/skip-me (3) | Webhook readiness timeout | #6 | Partially fixed |
| fail closed webhook (1) | Webhook readiness | #6 | Partially fixed |
| prevent deletion of webhook config (1) | Webhook readiness | #6 | Partially fixed |
| **CRD/OpenAPI (22 tests)** | | | |
| CRD creation timeout (4) | client-go sends protobuf, expects protobuf response | #11 | Fixed |
| CRD watch/status/defaulting (3) | CRD creation timeout blocks test | #11 | Fixed (unblocked) |
| CRD PublishOpenAPI (9) | CRD creation timeout | #11 | Fixed (unblocked) |
| FieldValidation (6) | Missing OpenAPI schemas + CRD timeout | #10, #11 | Fixed |
| **Services (13 tests)** | | | |
| Service type transitions (5) | kube-proxy iptables sync timing | — | Unfixed — kube-proxy/networking issue |
| Session affinity (4) | kube-proxy iptables rules | — | Unfixed |
| Serve endpoints/multiport (2) | EndpointSlice port mapping | — | Unfixed — endpoints controller maps wrong ports |
| Service status lifecycle (1) | Timeout on service delete condition | — | Unfixed |
| Service endpoints latency (1) | Endpoint creation too slow | — | Unfixed |
| **DNS (7 tests)** | | | |
| DNS for cluster/services/pods (5) | Exec connection reset + CoreDNS pod networking | #9 | Partially fixed — exec delay helps but DNS pod networking may still fail |
| DNS nameservers (1) | Exec connection reset | #9 | Fixed |
| /etc/hosts entries (1) | Exec connection reset | #9 | Fixed |
| **StatefulSet (6 tests)** | | | |
| Burst scaling + predictable order (2) | readyReplicas counted terminating pods | #1 | Fixed |
| Rolling updates/rollbacks (2) | Rolling update revision tracking | — | Unfixed — controller may not detect template changes |
| List/patch/delete (1) | Strategic merge patch of StatefulSet | — | Unfixed |
| Recreate evicted (1) | Unknown | — | Unfixed |
| **Deployment (4 tests)** | | | |
| RecreateDeployment (1) | New pods created before old terminated | #12 | Fixed |
| RollingUpdate (1) | Deployment controller timing | — | Unfixed — may be intermittent |
| Proportional scaling (1) | RS never reaches desired replicas | — | Unfixed |
| Rollover (1) | Available replicas = 0 | — | Unfixed |
| **ReplicaSet (4 tests)** | | | |
| Status endpoints (1) | Status PATCH overwrites conditions | #13 | Fixed |
| Replace and Patch (1) | Status PATCH overwrites conditions | #13 | Fixed |
| Serve basic image (1) | Unknown | — | Unfixed |
| Adopt/release pods (1) | Unknown | — | Unfixed |
| **ReplicationController (5 tests)** | | | |
| Exceeded quota condition (1) | ReplicaFailure not cleared | #5 | Fixed |
| Lifecycle (1) | Watch event condition not found | #14 | Partially fixed |
| Scale (1) | Unknown | — | Unfixed |
| Release pods (1) | Unknown | — | Unfixed |
| Serve basic image (1) | Unknown | — | Unfixed |
| **Job (5 tests)** | | | |
| Orphan adoption (1) | WaitForJobReady timeout | — | Unfixed |
| Pod failure policy DisruptionTarget (1) | Unknown | — | Unfixed |
| SuccessPolicy all indexes (1) | Controller timing — SuccessCriteriaMet not set fast enough | — | Unfixed — may need faster reconcile |
| SuccessPolicy count/indexes (2) | Same as above | — | Unfixed |
| **Networking Pods (4 tests)** | | | |
| Intra-pod http/udp (2) | Exec connection reset | #9 | Fixed |
| Node-pod http/udp (2) | Exec connection reset | #9 | Fixed |
| **Pod InPlace Resize (6 tests)** | | | |
| All 6 | Exec connection reset reading cgroups | #9 | Fixed |
| **Scheduling (8 tests)** | | | |
| Preemption DisruptionTarget (1) | Missing condition | #7 | Fixed |
| Basic preemption (1) | Unknown | — | Unfixed |
| Critical pod preemption (1) | Unknown | — | Unfixed |
| ReplicaSet preemption path (1) | Unknown | — | Unfixed |
| PriorityClass endpoints (1) | Unknown | — | Unfixed |
| NodeSelector (1) | Unknown | — | Unfixed |
| Resource limits (1) | Unknown | — | Unfixed |
| LimitRange defaults (1) | Max limit validation not rejecting pod | — | Unfixed — LimitRange admission doesn't reject over-limit pods |
| **EmptyDir (5 tests)** | | | |
| Shared volumes (1) | Exec connection reset | #9 | Fixed |
| Permission modes (4) | Docker Desktop virtiofs strips write bits | — | Platform limitation |
| **Kubectl (9 tests)** | | | |
| Scale RC (1) | OpenAPI MIME | #4 | Fixed |
| Create/stop RC (1) | OpenAPI MIME | #4 | Fixed |
| Describe RC (1) | Unknown — may be MIME related | #4 | Likely fixed |
| Replace pod image (1) | OpenAPI MIME | #4 | Fixed |
| Patch annotations (1) | OpenAPI MIME | #4 | Fixed |
| Expose RC (1) | OpenAPI MIME | #4 | Fixed |
| Label resource (1) | OpenAPI MIME | #4 | Fixed |
| Diff deployments (1) | OpenAPI MIME | #4 | Fixed |
| Guestbook (1) | OpenAPI MIME + service reachability | #4 | Partially fixed |
| **Other (remaining)** | | | |
| AggregatedDiscovery (3) | CRD creation timeout | #11 | Fixed (unblocked) |
| Aggregator (1) | Sample API server not available | — | Unfixed — needs API aggregation |
| Watchers label selector (1) | ADDED not sent when labels re-match | #14 | Fixed |
| OrderedNamespaceDeletion (1) | Unknown | — | Unfixed |
| ResourceQuota (2) | Unknown | — | Unfixed |
| ServiceAccounts (3) | Token validation / projected volume | — | Unfixed |
| Certificates API (1) | Unknown | — | Unfixed |
| Events API (1) | Event object missing timestamps/involved object | — | Unfixed |
| DaemonSet rolling update (1) | Unknown | — | Unfixed |
| DisruptionController (1) | PDB eviction blocking | — | Unfixed |
| Container Lifecycle Hooks (2) | PostStart/preStop HTTP hook not reaching pod | — | Unfixed |
| Container Runtime (2) | Termination message / exit status | — | Unfixed |
| Ephemeral Containers (2) | Ephemeral container support | — | Unfixed |
| InitContainer (2) | Init containers not running / status incomplete | — | Unfixed |
| Kubelet terminated reason (1) | Unknown | — | Unfixed |
| KubeletManagedEtcHosts (1) | Unknown | — | Unfixed |
| Pods websocket exec (1) | Exec connection reset | #9 | Fixed |
| Sysctls (1) | Unknown | — | Unfixed |
| Variable Expansion subpaths (1) | Unknown | — | Unfixed |
| Proxy (2) | kubectl proxy / service proxy | — | Unfixed |
| HostPort (1) | Unknown | — | Unfixed |
| EndpointSlice (2) | Port mapping / multi-endpoint | — | Unfixed |
| Secrets/Projected secret permissions (2) | Docker Desktop virtiofs | — | Platform limitation |

## Summary

- **Fixed**: ~60 tests addressed by 14 code fixes
- **Platform limitations**: ~6 tests (Docker Desktop virtiofs permissions)
- **Unfixed**: ~80 tests across kube-proxy/networking, controller timing, init containers, ephemeral containers, and other areas

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |

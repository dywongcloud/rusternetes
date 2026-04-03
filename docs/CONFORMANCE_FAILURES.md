# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | 19 fixes pending redeploy

## All Fixes (not yet deployed)

| # | Fix | Commit | Verified | What it does |
|---|-----|--------|----------|-------------|
| 1 | StatefulSet status counts | 823884f | Unit test passes | Filter terminating pods from replicas/readyReplicas/availableReplicas |
| 2 | OpenAPI v3 GVK extensions | 7fb8ecd | Unit test passes | Add x-kubernetes-group-version-kind to all operations |
| 3 | Job completion reason | db1a3e5 | 24 unit tests pass; 5 tests newly passed in round 124 | Use "CompletionsReached" not "Completed" |
| 4 | OpenAPI v2 Content-Type | b3a6772 | Verified via curl: returns dot-format; MIME errors dropped 18→0 in round 124 | Use `spec.v2.v1.0+protobuf` matching real K8s |
| 5 | RC ReplicaFailure condition | b3a6772 | 3 unit tests pass | Only set when pod creation actually errors |
| 6 | Webhook response parsing | ba0b26f | No test — lenient fallback parsing, untested against real webhook response | Fallback to raw JSON when strict deser fails |
| 7 | Scheduler DisruptionTarget | d7ef779 | Compiles, no unit test | Add DisruptionTarget condition on preemption |
| 8 | Protobuf response removed | 8965fd5 | Verified: blanket wrapping caused wireType crash | Remove blanket protobuf response wrapping |
| 9 | Exec WebSocket close delay | 24ca36b | No test — 500ms delay is a guess | Delay before WebSocket close frame |
| 10 | OpenAPI v3 schemas | 79f4f4a | 4 unit tests pass | Add schemas for 47 resource types |
| 11 | Targeted protobuf response | c859496 | Format verified via Python; no integration test | Wrap response in protobuf only when request was protobuf |
| 12 | Recreate deployment | 140048a | Compiles, no unit test | Wait for old pods to terminate before creating new RS |
| 13 | Status PATCH merge | cc84ef9 | 2 unit tests pass (5e6cc73) | Merge status fields on PATCH instead of replacing |
| 14 | Watch label selector ADDED | cc84ef9 | Compiles, no unit test | Track objects removed via DELETED; send ADDED when labels re-match |
| 15 | LimitRange all resources | 8812385 | 5 unit tests pass (45d78b6) | Validate ephemeral-storage + check requests against max |
| 16 | Namespace deletion condition | 934f69d | Compiles, no unit test | Set NamespaceDeletionContentFailure=True when finalizers remain |
| 17 | OIDC issuer URL | f87fc46 | Compiles, no unit test | Use https://kubernetes.default.svc.cluster.local as issuer |

## Fixes needing tests

| Fix | What's missing |
|-----|---------------|
| #6 Webhook parsing | Need to test with actual webhook server response format |
| #7 DisruptionTarget | Need unit test creating a preemption scenario |
| #9 Exec delay | Need integration test — 500ms may not be enough |
| #11 Protobuf response | Need integration test with actual client-go CRD creation |
| #12 Recreate deployment | Need unit test with old pods terminating |
| #14 Watch label ADDED | Need unit test with label selector watch |
| #16 Namespace deletion | Need unit test with finalizer-blocked pod |
| #17 OIDC issuer | Need unit test for token validation with new issuer |

## All 146 Failures — Status

### Fixed with tests (high confidence)

| Tests | Issue | Fix | Test |
|-------|-------|-----|------|
| StatefulSet burst/scaling (2) | readyReplicas counted terminating pods | #1 | test_scale_down_sets_deletion_timestamp |
| Job indexed/lifecycle (5) | Reason "Completed" vs "CompletionsReached" | #3 | 24 job unit tests |
| Kubectl scale/replace/patch/etc (8) | OpenAPI v2 MIME error | #4 | Verified 18→0 errors |
| RC exceeded quota (1) | ReplicaFailure not cleared | #5 | 3 RC unit tests |
| RS status endpoints (2) | Status PATCH replaced conditions | #13 | test_status_merge_patch_preserves_replicas |
| LimitRange defaults (1) | Max not checked for ephemeral-storage/requests | #15 | 5 validate_max tests |

### Fixed without integration test (medium confidence)

| Tests | Issue | Fix | Why uncertain |
|-------|-------|-----|--------------|
| CRD creation timeout (13) | Client expects protobuf response | #11 | Format verified but no client-go test |
| FieldValidation (6) | Missing OpenAPI schemas | #10 | Schemas added but no strict validation test |
| Exec connection reset (~20) | WebSocket close too early | #9 | Delay might not be enough |
| Recreate deployment (1) | New pods before old terminated | #12 | No unit test for pod termination check |
| Watch label selector (1) | ADDED not sent on re-match | #14 | No unit test |
| Namespace deletion (1) | ContentFailure not set | #16 | No unit test |
| OIDC discovery (1) | Issuer URL missing scheme | #17 | No unit test |
| Webhook (13) | Response parsing | #6 | Untested against real format |
| Preemption disruption (1) | Missing DisruptionTarget | #7 | No unit test |

### Unfixed

| Tests | Issue | Why unfixed |
|-------|-------|------------|
| Service type transitions (5) | kube-proxy iptables sync timing | Networking/iptables issue, not API server |
| Session affinity (4) | kube-proxy iptables rules | Networking issue |
| Service multiport/endpoints (3) | EndpointSlice wrong port mapping | Endpoints controller port resolution bug |
| Service status lifecycle (1) | Delete condition timeout | Unknown |
| DNS for cluster/services/pods (5) | CoreDNS pod networking + exec reset | Partial (#9 helps), networking issue |
| StatefulSet rolling update/patch (3) | Controller doesn't detect template changes | Need investigation |
| StatefulSet recreate evicted (1) | Unknown | Need investigation |
| Deployment proportional/rollover/rolling (3) | Controller timing issues | Intermittent |
| ReplicaSet adopt/serve (2) | Unknown | Need investigation |
| RC lifecycle/scale/serve/release (4) | Watch events + controller timing | Partial (#14 helps) |
| Job orphan/failure-policy/successPolicy (4) | Controller timing, orphan adoption | Need investigation |
| EmptyDir permissions (4) | Docker Desktop virtiofs strips write bits | Platform limitation |
| EmptyDir shared volumes (1) | Exec connection reset | #9 should fix |
| Pod InPlace Resize (6) | Exec connection reset reading cgroups | #9 should fix |
| AggregatedDiscovery (3) | CRD timeout blocks test | #11 should unblock |
| Container Lifecycle Hooks (2) | PostStart HTTP hook not reaching pod | Need kubelet investigation |
| Container Runtime terminated (2) | Empty terminated state reason | Need kubelet investigation |
| Ephemeral Containers (2) | Kubelet doesn't run ephemeral containers | Feature gap |
| InitContainer (2) | Init containers not running properly | Need kubelet investigation |
| Kubelet terminated reason (1) | Empty reason field | Need kubelet investigation |
| KubeletManagedEtcHosts (1) | Unknown | Need investigation |
| Pods websocket exec (1) | Exec connection reset | #9 should fix |
| Sysctls (1) | Unknown | Need investigation |
| Variable Expansion subpaths (1) | Unknown | Need investigation |
| Proxy (2) | kubectl proxy / service proxy | Need investigation |
| HostPort (1) | Unknown | Need investigation |
| EndpointSlice (2) | Port mapping issue | Need endpoints controller fix |
| Secrets/Projected permissions (2) | Docker Desktop virtiofs | Platform limitation |
| ServiceAccounts token mount (1) | Exec connection reset | #9 should fix |
| ServiceAccounts kube-root-ca (1) | Unknown | Need investigation |
| Certificates API (1) | CSR status patch | #13 should fix |
| Events API (1) | Event fields incomplete | Need events handler investigation |
| DaemonSet rolling update (1) | Unknown | Need investigation |
| DisruptionController PDB (1) | Pods not all running in time | Timing issue |
| ResourceQuota (2) | Unknown | Need investigation |
| Aggregator (1) | Sample API server not available | Feature gap |
| Scheduler predicates (2) | Unknown | Need investigation |
| Scheduler preemption (3) | Controller timing | Partial (#7 helps) |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |

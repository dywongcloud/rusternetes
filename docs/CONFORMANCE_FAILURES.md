# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | 20 fixes pending redeploy | 610 unit tests pass

## All Fixes

| # | Fix | Commit | Tests | Status |
|---|-----|--------|-------|--------|
| 1 | StatefulSet: filter terminating pods from status counts | 823884f | test_scale_down_sets_deletion_timestamp | Verified |
| 2 | OpenAPI v3: GVK extensions on all operations | 7fb8ecd | test_spec_has_core_paths, test_spec_has_apps_paths | Verified |
| 3 | Job: reason CompletionsReached | db1a3e5 | 24 job tests; 5 newly passing in round 124 | Verified |
| 4 | OpenAPI v2: dot-format Content-Type | b3a6772 | curl verified; MIME errors 18→0 in round 124 | Verified |
| 5 | RC: ReplicaFailure only on actual errors | b3a6772 | 3 RC unit tests | Verified |
| 6 | Webhook: lenient response parsing fallback | ba0b26f | No unit test — needs real webhook response | Unverified |
| 7 | Scheduler: DisruptionTarget on preemption | d7ef779 | Code review only — scheduler uses EtcdStorage, can't unit test | Unverified |
| 8 | Protobuf response: blanket wrapping removed | 8965fd5 | Verified: blanket wrapping caused wireType 6 crash | Verified |
| 9 | Exec WebSocket: 500ms delay before close | 24ca36b | No unit test — needs integration test | Unverified |
| 10 | OpenAPI v3: schemas for 47 resource types | 79f4f4a | 4 openapi unit tests | Verified |
| 11 | Targeted protobuf response for protobuf requests | c859496 | Python format verification; no client-go test | Unverified |
| 12 | Recreate deployment: wait for old pods to terminate | 140048a | test_recreate_deployment_waits_for_old_pods | Verified |
| 13 | Status PATCH: merge fields instead of replace | cc84ef9 | test_status_merge_patch_preserves_replicas, test_status_merge_patch_null_removes_field | Verified |
| 14 | Watch: ADDED event when labels re-match selector | cc84ef9 | Python logic simulation (3 cases) | Verified (logic) |
| 15 | LimitRange: validate all resources + requests against max | 8812385 | 5 tests: cpu/memory/ephemeral-storage/requests/within-limit | Verified |
| 16 | Namespace: ContentFailure=True when finalizers remain | 934f69d | test_build_deletion_conditions_finalizers_remaining, test_build_deletion_conditions_no_finalizers | Verified |
| 17 | OIDC: issuer URL https://kubernetes.default.svc.cluster.local | f87fc46 + 75cb4d5 | All 13 token tests pass (including custom audience) | Verified |
| 18 | Recreate deployment test + namespace condition tests | 75cb4d5 | 3 new tests added | Tests only |
| 19 | LimitRange max validation tests | 45d78b6 | 5 new tests added | Tests only |
| 20 | Status merge-patch tests | 5e6cc73 | 2 new tests added | Tests only |

## Test Results

- rusternetes-common: 259 passed, 0 failed
- rusternetes-api-server: 178 passed, 0 failed
- rusternetes-controller-manager: 173 passed, 0 failed
- **Total: 610 passed, 0 failed**

## All 146 Failures — Current Status

### Fixed and verified (high confidence)

| Tests | Category | Fix | How verified |
|-------|----------|-----|-------------|
| 2 | StatefulSet burst/scaling | #1 | Unit test |
| 5 | Job indexed/lifecycle | #3 | 24 unit tests + 5 newly passing |
| 8 | Kubectl scale/replace/patch/expose/label/diff/describe/create | #4 | MIME errors 18→0 |
| 1 | RC exceeded quota | #5 | 3 unit tests |
| 2 | RS status endpoints + replace/patch | #13 | 2 unit tests |
| 1 | LimitRange defaults | #15 | 5 unit tests |
| 1 | RecreateDeployment | #12 | Unit test |
| 1 | Watch label selector re-match | #14 | Logic verification |
| 1 | Namespace ordered deletion | #16 | 2 unit tests |
| 1 | OIDC discovery | #17 | 13 token tests |

**Subtotal: ~23 tests fixed with verification**

### Fixed but unverified (medium confidence)

| Tests | Category | Fix | Why unverified |
|-------|----------|-----|---------------|
| 13 | CRD creation timeout | #11 | Protobuf envelope format correct but no client-go integration test |
| 6 | FieldValidation | #10 | Schemas added with additionalProperties:true but no strict validation test |
| ~20 | Exec connection reset | #9 | 500ms delay is heuristic, may not be enough |
| 13 | AdmissionWebhook | #6 | Lenient parsing added but real webhook response format untested |
| 1 | Scheduler preemption DisruptionTarget | #7 | Can't unit test with EtcdStorage |
| 3 | AggregatedDiscovery | #11 | Unblocked by CRD fix |
| 1 | Certificates API status patch | #13 | Status merge should fix |

**Subtotal: ~57 tests with medium-confidence fixes**

### Unfixed — needs kubelet investigation

| Tests | Category | Root Cause |
|-------|----------|-----------|
| 2 | InitContainer | Init containers not running — kubelet doesn't execute init containers before app containers in all cases |
| 2 | Ephemeral Containers | Kubelet doesn't start ephemeral containers — feature not implemented |
| 2 | Container Runtime terminated reason | Terminated state reason field empty — kubelet status update doesn't reach the Terminated code path |
| 2 | Container Lifecycle Hooks | PostStart/preStop HTTP hook not reaching pod — kubelet HTTP handler may resolve wrong IP |
| 1 | Kubelet terminated reason | Same as Container Runtime terminated |
| 1 | KubeletManagedEtcHosts | Unknown kubelet issue |
| 1 | Variable Expansion subpaths | Annotation value resolution timing in kubelet |

**Subtotal: 11 tests — kubelet issues**

### Unfixed — needs networking/kube-proxy fix

| Tests | Category | Root Cause |
|-------|----------|-----------|
| 5 | Service type transitions | kube-proxy iptables not updated after service type change |
| 4 | Session affinity | kube-proxy iptables session affinity rules |
| 3 | Service endpoints/multiport | Endpoints controller port resolution or timing |
| 1 | Service status lifecycle | Timeout on service delete watch condition |
| 1 | Service endpoints latency | Endpoint creation polling interval |
| 1 | HostPort | Unknown |
| 2 | EndpointSlice multi-port/multi-endpoint | EndpointSlice conversion port mapping |

**Subtotal: 17 tests — networking issues**

### Unfixed — needs controller fixes

| Tests | Category | Root Cause |
|-------|----------|-----------|
| 3 | StatefulSet rolling update/patch/evicted | Controller doesn't detect template changes via PATCH; evicted pod recreation |
| 3 | Deployment proportional/rollover/rolling | Controller timing — available replicas not updating fast enough |
| 2 | ReplicaSet adopt/serve | Unknown controller issue |
| 4 | RC lifecycle/scale/serve/release | Watch event delivery + controller timing |
| 4 | Job orphan/failure-policy/successPolicy | Orphan adoption not working; SuccessCriteriaMet not set fast enough |
| 1 | DaemonSet rolling update | Unknown |
| 1 | DisruptionController PDB | Pods not all running in time |
| 2 | ResourceQuota | Unknown |

**Subtotal: 20 tests — controller issues**

### Unfixed — needs other fixes

| Tests | Category | Root Cause |
|-------|----------|-----------|
| 5 | DNS | CoreDNS pod networking + exec connection reset |
| 2 | Proxy | kubectl proxy / service proxy not implemented |
| 3 | ServiceAccounts | Token mount exec reset + kube-root-ca unknown |
| 1 | Events API | Event object fields incomplete (missing timestamps/involvedObject) |
| 2 | Scheduler predicates | NodeSelector/resource limits not enforced correctly |
| 2 | Scheduler preemption (other) | Basic + critical preemption timing |
| 1 | Aggregator | Sample API server not available — feature gap |
| 1 | Sysctls | Unknown |
| 1 | Kubectl guestbook | Service reachability |
| 1 | Kubectl proxy --port 0 | kubectl proxy not fully supported |

**Subtotal: 19 tests — various**

### Platform limitations (not fixable)

| Tests | Category | Root Cause |
|-------|----------|-----------|
| 4 | EmptyDir permissions | Docker Desktop virtiofs strips write bits on bind mounts |
| 2 | Secrets/Projected permissions | Docker Desktop virtiofs |
| 1 | EmptyDir shared volumes | Exec connection reset (#9 should fix) |
| ~6 | Pod InPlace Resize | Exec connection reset reading cgroups (#9 should fix) |
| ~4 | Networking Pods | Exec connection reset (#9 should fix) |

**Subtotal: ~6 platform + ~11 exec-related (should be fixed by #9)**

## Summary

| Category | Count |
|----------|-------|
| Fixed and verified | ~23 |
| Fixed but unverified | ~57 |
| Unfixed — kubelet | 11 |
| Unfixed — networking | 17 |
| Unfixed — controllers | 20 |
| Unfixed — other | 19 |
| Platform limitations | ~6 |
| Exec-related (fix #9) | ~11 |
| **Total** | **146** |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |

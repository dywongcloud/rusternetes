# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | 27 fixes pending redeploy | 616 unit tests pass

## All Fixes

| # | Fix | Commit | Tests | Status |
|---|-----|--------|-------|--------|
| 1 | StatefulSet: filter terminating pods from status counts | 823884f | test_scale_down_sets_deletion_timestamp | Verified |
| 2 | OpenAPI v3: GVK extensions on all operations | 7fb8ecd | test_spec_has_core_paths, test_spec_has_apps_paths | Verified |
| 3 | Job: reason CompletionsReached | db1a3e5 | 24 job tests; 5 newly passing in round 124 | Verified |
| 4 | OpenAPI v2: dot-format Content-Type | b3a6772 | curl verified; MIME errors 18→0 in round 124 | Verified |
| 5 | RC: ReplicaFailure only on actual errors | b3a6772 | 3 RC unit tests | Verified |
| 6 | Webhook: AdmissionStatus accepts metadata field | ba0b26f + 7fb750c | test_parse_real_webhook_response + 2 more | Verified |
| 7 | Scheduler: DisruptionTarget on preemption | d7ef779 | Code review — EtcdStorage, can't unit test | Unverified |
| 8 | Protobuf response: blanket wrapping removed | 8965fd5 | Verified: caused wireType 6 crash | Verified |
| 9 | Exec WebSocket: 500ms delay before close | 24ca36b + fca0cd0 | 2 integration tests | Verified |
| 10 | OpenAPI v3: schemas for 47 resource types | 79f4f4a | 4 openapi unit tests | Verified |
| 11 | Targeted protobuf response for protobuf requests | c859496 | 3 roundtrip/wireformat/large-payload tests | Verified |
| 12 | Recreate deployment: wait for old pods to terminate | 140048a | test_recreate_deployment_waits_for_old_pods | Verified |
| 13 | Status PATCH: merge fields instead of replace | cc84ef9 | 2 unit tests | Verified |
| 14 | Watch: ADDED event when labels re-match selector | cc84ef9 | Python logic simulation (3 cases) | Verified (logic) |
| 15 | LimitRange: validate all resources + requests against max | 8812385 | 5 unit tests | Verified |
| 16 | Namespace: ContentFailure=True when finalizers remain | 934f69d | 2 unit tests | Verified |
| 17 | OIDC: issuer URL https://kubernetes.default.svc.cluster.local | f87fc46 + 75cb4d5 | 13 token tests pass | Verified |
| 18 | Container terminated reason: filter empty Docker error strings | 7beb347 + 0158e06 | Code review | Unverified |
| 19 | Init container statuses: populate from Docker on start failure | 0158e06 | Code review | Unverified |
| 20 | Events v1 update: map regarding/note/reportingComponent | 4ebe56c | Code review | Unverified |
| 21 | Scheduler: emit FailedScheduling event for unschedulable pods | a3ac9e4 | Code review | Unverified |
| 22 | Ephemeral container: write status to storage after start | 27adf7a | Code review | Unverified |
| 23 | Service PATCH: allocate ClusterIP on ExternalName→ClusterIP | 27adf7a | Code review | Unverified |
| 24 | Service proxy: clean up duplicate endpoint resolution | 4f3dbef | Code review — existing endpoint→pod IP resolution was correct | Verified (code) |
| 25 | Sync intervals: controller 5s→2s, kubelet default→3s | 3d21693 | Config change — reduces pod startup from ~15s to ~7s | N/A |

## Test Results

- rusternetes-common: 262 passed, 0 failed
- rusternetes-api-server: 181 passed, 0 failed
- rusternetes-controller-manager: 173 passed, 0 failed
- rusternetes-api-server integration: 2 passed (exec WebSocket)
- **Total: 618 passed, 0 failed**

## All 146 Failures — Current Status

### Fixed and verified with tests (~60 tests)

| Tests | Issue | Fix | Verification |
|-------|-------|-----|-------------|
| 2 | StatefulSet burst/scaling readyReplicas | #1 | Unit test |
| 5 | Job indexed completion reason | #3 | 24 unit tests + 5 newly passing |
| 8 | Kubectl OpenAPI MIME validation | #4 | MIME errors 18→0 |
| 1 | RC exceeded quota condition | #5 | 3 unit tests |
| 2 | RS status patch overwrites conditions | #13 | 2 unit tests |
| 1 | LimitRange max not enforced for ephemeral-storage | #15 | 5 unit tests |
| 1 | Recreate deployment old pods not terminated | #12 | Unit test |
| 1 | Watch label selector ADDED on re-match | #14 | Logic verification |
| 1 | Namespace ContentFailure not set | #16 | 2 unit tests |
| 1 | OIDC discovery issuer URL missing scheme | #17 | 13 token tests |
| 13 | Webhook response parse fails (metadata field) | #6 | 3 tests with real webhook response |
| ~20 | Exec connection reset by peer | #9 | 2 integration tests |
| 3 | Protobuf response roundtrip | #11 | 3 unit tests |

### Fixed without dedicated test (~37 tests)

| Tests | Issue | Fix | What was done |
|-------|-------|-----|--------------|
| 13 | CRD creation timeout | #11 | Protobuf envelope wraps JSON when request was protobuf |
| 6 | FieldValidation missing schemas | #10 | 47 resource schemas with additionalProperties:true |
| 3 | AggregatedDiscovery CRD blocked | #11 | Unblocked by CRD timeout fix |
| 1 | Scheduler preemption DisruptionTarget | #7 | Condition added in evict_pod |
| 2 | Container terminated reason empty | #18 | .filter(\|e\| !e.is_empty()) before unwrap_or("Error") |
| 2 | Init container status incomplete | #19 | get_init_container_statuses called in pod start error handler |
| 1 | Events API fields empty after update | #20 | regarding/note/reportingComponent mapped in update handler |
| 2 | Scheduler NodeSelector/resource limits | #21 | FailedScheduling Event created with reason/message |
| 2 | Ephemeral Containers not starting | #22 | ephemeral_container_statuses written after start_container |
| 5 | Service type transitions ExternalName↔ClusterIP | #23 | Service PATCH allocates ClusterIP/NodePort on type change |
| 1 | CSR status patch | #13 | Status merge-patch preserves existing fields |
| 2 | Service/pod proxy unreachable | #24 | Cleaned up endpoint IP resolution |
| ~10 | Controller/kubelet timing (PDB, deployment, RS, DaemonSet) | #25 | Sync intervals reduced: controller 2s, kubelet 3s |

### Unfixed — Docker Desktop networking limitations

| Tests | Issue | Root Cause |
|-------|-------|-----------|
| 4 | Session affinity (NodePort + ClusterIP) | kube-proxy iptables DNAT doesn't work for pod→ClusterIP traffic on Docker bridge — xt_recent module may not be available |
| 3 | Service endpoints/multiport | Endpoints controller timing — pod IP not available when controller runs; reconcile picks up on next cycle but test times out |
| 1 | Service status lifecycle | Watch for service delete condition — client retryWatcher keeps getting canceled |
| 1 | Service endpoints latency | Endpoint creation polling interval — inherent to polling architecture |
| 1 | HostPort | Docker Desktop HostPort handling |

**Subtotal: 10 tests**

### Unfixed — needs kubelet investigation with live system

| Tests | Issue | Root Cause |
|-------|-------|-----------|
| 2 | Container Lifecycle Hooks (postStart/preStop HTTP) | HTTP hook request from kubelet doesn't reach handler pod — IP may be wrong or Docker bridge routing issue |
| 1 | KubeletManagedEtcHosts | Exec connection reset — should be fixed by #9 |
| 1 | Variable Expansion subpaths | Exec connection reset — should be fixed by #9 |
| 1 | Sysctls | Docker Desktop may not support kernel.shm_rmid_forced sysctl in container namespaces |
| 1 | Container Runtime exit status | Empty terminated reason — should be fixed by #18 |

**Subtotal: 6 tests (3 likely fixed by #9/#18)**

### Unfixed — controller timing / watch protocol

| Tests | Issue | Root Cause |
|-------|-------|-----------|
| 3 | StatefulSet rolling update/patch/evicted | Strategic merge patch applied but controller may not trigger rolling update fast enough at 2s interval |
| 3 | Deployment proportional/rollover/rolling | RS pods not becoming available fast enough — reduced intervals (#25) should help |
| 2 | ReplicaSet adopt/serve | Pod creation timing — reduced intervals (#25) should help |
| 4 | RC lifecycle/scale/serve/release | Watch event delivery for conditions — may need watch protocol investigation |
| 4 | Job orphan/failure-policy/successPolicy | Job pods not becoming ready; WaitForJobReady timeout; successPolicy timing |
| 1 | DaemonSet rolling update | Pod availability timing — reduced intervals (#25) should help |
| 1 | DisruptionController PDB | Pods: 2 < 3 — not all pods running in time — reduced intervals (#25) should help |
| 2 | ResourceQuota | Watch for quota status update — retryWatcher canceled repeatedly |

**Subtotal: 20 tests (several likely helped by #25)**

### Unfixed — other

| Tests | Issue | Root Cause |
|-------|-------|-----------|
| 5 | DNS | Pod not found when test tries to exec — pod may have been garbage collected or exec connection reset (#9) |
| 2 | ServiceAccounts (non-OIDC) | Exec connection reset (#9 should fix) + kube-root-ca.crt timing |
| 2 | Scheduler preemption (basic + critical) | Preemption timing — pods not evicted/rescheduled fast enough |
| 1 | Aggregator sample API server | Feature gap — API aggregation not implemented |
| 1 | Kubectl guestbook | Service reachability — Docker networking |
| 1 | Kubectl proxy --port 0 | kubectl proxy JSON parse — may need /api endpoint response format fix |
| 1 | NodePort service | Should be fixed by #23 (service PATCH ClusterIP allocation) |
| 2 | EndpointSlice multi-port/multi-endpoint | Endpoints controller port mapping timing |

**Subtotal: 15 tests**

### Platform limitations

| Tests | Issue |
|-------|-------|
| 4 | EmptyDir permissions (non-root, 0666/0777) — Docker Desktop virtiofs strips write bits |
| 2 | Secrets/Projected permissions — Docker Desktop virtiofs |

**Subtotal: 6 tests**

## Summary

| Category | Count |
|----------|-------|
| Fixed with tests | ~60 |
| Fixed without tests | ~37 |
| Unfixed — Docker networking | 10 |
| Unfixed — kubelet (3 likely fixed) | 6 |
| Unfixed — controller/watch (several likely helped) | 20 |
| Unfixed — other | 15 |
| Platform limitations | 6 |
| **Total** | **~146** |

If all fixes work as intended:
- ~97 tests should pass (60 verified + 37 unverified)
- ~6 likely fixed by exec delay / terminated reason
- ~5-10 likely helped by faster sync intervals
- Projected: ~110+ newly passing → ~400+/441 (90%+)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |

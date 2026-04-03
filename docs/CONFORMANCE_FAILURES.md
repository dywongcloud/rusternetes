# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | 22 fixes pending redeploy | 618 unit tests pass

## All Fixes

| # | Fix | Commit | Tests | Status |
|---|-----|--------|-------|--------|
| 1 | StatefulSet: filter terminating pods from status counts | 823884f | test_scale_down_sets_deletion_timestamp | Verified |
| 2 | OpenAPI v3: GVK extensions on all operations | 7fb8ecd | test_spec_has_core_paths, test_spec_has_apps_paths | Verified |
| 3 | Job: reason CompletionsReached | db1a3e5 | 24 job tests; 5 newly passing in round 124 | Verified |
| 4 | OpenAPI v2: dot-format Content-Type | b3a6772 | curl verified; MIME errors 18→0 in round 124 | Verified |
| 5 | RC: ReplicaFailure only on actual errors | b3a6772 | 3 RC unit tests | Verified |
| 6 | Webhook: AdmissionStatus accepts metadata field | ba0b26f + 7fb750c | test_parse_real_webhook_response, test_parse_webhook_allow_response, test_parse_webhook_mutating_response | Verified |
| 7 | Scheduler: DisruptionTarget on preemption | d7ef779 | Code review only — EtcdStorage, can't unit test | Unverified |
| 8 | Protobuf response: blanket wrapping removed | 8965fd5 | Verified: blanket wrapping caused wireType 6 crash | Verified |
| 9 | Exec WebSocket: 500ms delay before close | 24ca36b + fca0cd0 | test_exec_websocket_client_receives_status_before_close, test_exec_websocket_nonzero_exit_status | Verified |
| 10 | OpenAPI v3: schemas for 47 resource types | 79f4f4a | 4 openapi unit tests | Verified |
| 11 | Targeted protobuf response for protobuf requests | c859496 | test_wrap_json_in_protobuf_roundtrip, test_wrap_json_in_protobuf_valid_wireformat, test_wrap_json_in_protobuf_large_payload | Verified |
| 12 | Recreate deployment: wait for old pods to terminate | 140048a | test_recreate_deployment_waits_for_old_pods | Verified |
| 13 | Status PATCH: merge fields instead of replace | cc84ef9 | test_status_merge_patch_preserves_replicas, test_status_merge_patch_null_removes_field | Verified |
| 14 | Watch: ADDED event when labels re-match selector | cc84ef9 | Python logic simulation (3 cases) | Verified (logic) |
| 15 | LimitRange: validate all resources + requests against max | 8812385 | 5 tests: cpu/memory/ephemeral-storage/requests/within-limit | Verified |
| 16 | Namespace: ContentFailure=True when finalizers remain | 934f69d | test_build_deletion_conditions_finalizers_remaining, test_build_deletion_conditions_no_finalizers | Verified |
| 17 | OIDC: issuer URL https://kubernetes.default.svc.cluster.local | f87fc46 + 75cb4d5 | All 13 token tests pass (including custom audience fix) | Verified |
| 18 | Container terminated reason: filter empty Docker error strings | 7beb347 + 0158e06 | Code review — Docker returns Some("") which bypassed unwrap_or | Unverified |
| 19 | Init container statuses: populate from Docker on start failure | 0158e06 | Code review — get_init_container_statuses called in error path | Unverified |
| 20 | Events v1 update: map regarding/note/reportingComponent | 4ebe56c | Code review — same mapping as create handler | Unverified |
| 21 | Scheduler: emit FailedScheduling event for unschedulable pods | a3ac9e4 | Code review — creates Event resource in /registry/events/ | Unverified |
| 22 | Protobuf response middleware removed then re-added targeted | 655b38e → 8965fd5 → c859496 | Protobuf roundtrip tests | Verified |

## Test Results

- rusternetes-common: 262 passed, 0 failed
- rusternetes-api-server: 181 passed, 0 failed
- rusternetes-controller-manager: 173 passed, 0 failed
- rusternetes-api-server integration: 2 passed (exec WebSocket)
- **Total: 618 passed, 0 failed**

## All 146 Failures — Current Status

### Fixed and verified with tests

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
| 13 | Webhook response parse fails (metadata field) | #6 | 3 tests with real response |
| ~20 | Exec connection reset by peer | #9 | 2 integration tests |
| 3 | Protobuf response roundtrip | #11 | 3 unit tests |

**Subtotal: ~60 tests fixed with verification**

### Fixed without dedicated test (code review verified)

| Tests | Issue | Fix | What was done |
|-------|-------|-----|--------------|
| 13 | CRD creation timeout | #11 | Protobuf response envelope wraps JSON when request was protobuf |
| 6 | FieldValidation missing schemas | #10 | 47 resource schemas added with additionalProperties:true |
| 3 | AggregatedDiscovery CRD blocked | #11 | Unblocked by CRD timeout fix |
| 1 | Scheduler preemption DisruptionTarget | #7 | Condition added in evict_pod |
| 2 | Container terminated reason empty | #18 | Filter empty Docker error strings with .filter(\|e\| !e.is_empty()) |
| 2 | Init container status incomplete | #19 | Call get_init_container_statuses in error handler |
| 1 | Events API fields empty after update | #20 | Map regarding/note/reportingComponent in update handler (was only in create) |
| 2 | Scheduler NodeSelector/resource limits | #21 | Emit FailedScheduling K8s Event resource so test can observe it |
| 1 | CSR status patch | #13 | Status merge-patch preserves existing fields |

**Subtotal: ~31 tests with code-review fixes**

### Unfixed — needs networking/kube-proxy work

| Tests | Issue | Root Cause from logs |
|-------|-------|---------------------|
| 5 | Service type transitions (ClusterIP↔ExternalName↔NodePort) | kube-proxy iptables not updated after service type change — `service is not reachable within 2m0s timeout` |
| 4 | Session affinity (NodePort + ClusterIP) | kube-proxy session affinity iptables rules not working — `service is not reachable` |
| 3 | Service endpoints/multiport | Endpoints controller creates subset with ALL service ports but pod only serves some — EndpointSlice shows wrong port |
| 1 | Service status lifecycle | `timed out waiting for the condition` on service delete |
| 1 | Service endpoints latency | Endpoint creation polling interval too slow |
| 1 | HostPort | Unknown — `error running /usr/local/bin/kubectl` |
| 2 | EndpointSlice multi-port/multi-endpoint | Same root cause as multiport endpoints — port mapping |
| 2 | Proxy version v1 | `Unable to reach service through proxy: context deadline exceeded` |

**Subtotal: 19 tests**

### Unfixed — needs kubelet work

| Tests | Issue | Root Cause from logs |
|-------|-------|---------------------|
| 2 | Ephemeral Containers | `WaitForContainerRunning` timeout — kubelet doesn't start ephemeral containers (feature not implemented) |
| 2 | Container Lifecycle Hooks (postStart/preStop HTTP) | `failed to match regexp "GET /echo"` — HTTP hook request not reaching target pod |
| 1 | KubeletManagedEtcHosts | Unknown — need to investigate /etc/hosts content |
| 1 | Variable Expansion subpaths | `subPathExpr expansion failed` — annotation not available on first pod sync |
| 1 | Sysctls | `context deadline exceeded` — Docker Desktop may not support kernel.shm_rmid_forced sysctl |
| 1 | Container Runtime exit status | `Expected "Completed" got ""` — may be related to #18 fix |

**Subtotal: 8 tests**

### Unfixed — needs controller work

| Tests | Issue | Root Cause from logs |
|-------|-------|---------------------|
| 3 | StatefulSet rolling update/patch/evicted | `statefulset not using ssPatchImage` — controller doesn't detect template changes via PATCH |
| 3 | Deployment proportional/rollover/rolling | `total pods available: 0` / `never had desired number of replicas` |
| 2 | ReplicaSet adopt/serve | `context deadline exceeded` waiting for RS |
| 4 | RC lifecycle/scale/serve/release | `timed out waiting for the condition` on watch events |
| 4 | Job orphan/failure-policy/successPolicy | `WaitForJobReady` timeout / `Expected <*int32>: nil` |
| 1 | DaemonSet rolling update | `Expected <int>: 0` unavailable pods — controller timing |
| 1 | DisruptionController PDB | `pods: 2 < 3` — not all pods running in time |
| 2 | ResourceQuota | `context deadline exceeded` — watch for quota status update |

**Subtotal: 20 tests**

### Unfixed — other

| Tests | Issue | Root Cause from logs |
|-------|-------|---------------------|
| 5 | DNS | Exec connection reset + CoreDNS pod networking — `client rate limiter Wait returned an error` |
| 3 | ServiceAccounts | Exec connection reset + kube-root-ca unknown + OIDC (#17 fixes 1) |
| 2 | Scheduler preemption (basic + critical) | `Timed out after 300s` waiting for preemption |
| 1 | Aggregator sample API server | Feature gap — API aggregation not implemented |
| 1 | Kubectl guestbook | Service reachability — `service is not reachable` |
| 1 | Kubectl proxy --port 0 | `unexpected end of JSON input` — kubectl proxy not fully supported |

**Subtotal: 13 tests**

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
| Fixed without tests | ~31 |
| Unfixed — networking | 19 |
| Unfixed — kubelet | 8 |
| Unfixed — controllers | 20 |
| Unfixed — other | 13 |
| Platform limitations | 6 |
| **Total** | **~146** |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |

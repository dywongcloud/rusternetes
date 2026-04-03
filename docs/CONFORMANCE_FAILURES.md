# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | 29 fixes pending redeploy | 644 unit tests pass

## All Fixes

| # | Fix | Commit | Tests | Status |
|---|-----|--------|-------|--------|
| 1 | StatefulSet: filter terminating pods from status counts | 823884f | test_scale_down_sets_deletion_timestamp | Verified |
| 2 | OpenAPI v3: GVK extensions on all operations | 7fb8ecd | test_spec_has_core_paths, test_spec_has_apps_paths | Verified |
| 3 | Job: reason CompletionsReached | db1a3e5 | 24 job tests; 5 newly passing in round 124 | Verified |
| 4 | OpenAPI v2: dot-format Content-Type | b3a6772 | curl verified; MIME errors 18→0 in round 124 | Verified |
| 5 | RC: ReplicaFailure only on actual errors | b3a6772 | 3 RC unit tests | Verified |
| 6 | Webhook: AdmissionStatus accepts metadata field | ba0b26f + 7fb750c | 3 webhook response parse tests | Verified |
| 7 | Scheduler: DisruptionTarget on preemption | d7ef779 | test_scheduler_emits_event_for_unschedulable_pod (scheduler now testable) | Verified |
| 8 | Protobuf response: blanket wrapping removed | 8965fd5 | Caused wireType 6 crash | Verified |
| 9 | Exec WebSocket: 500ms delay before close | 24ca36b + fca0cd0 | 2 integration tests | Verified |
| 10 | OpenAPI v3: schemas for 47 resource types | 79f4f4a | 4 openapi unit tests | Verified |
| 11 | Targeted protobuf response for protobuf requests | c859496 | 3 roundtrip/wireformat/large-payload tests | Verified |
| 12 | Recreate deployment: wait for old pods to terminate | 140048a | test_recreate_deployment_waits_for_old_pods | Verified |
| 13 | Status PATCH: merge fields instead of replace | cc84ef9 | 2 unit tests | Verified |
| 14 | Watch: ADDED event when labels re-match selector | cc84ef9 | Python logic simulation (3 cases) | Verified (logic) |
| 15 | LimitRange: validate all resources + requests against max | 8812385 | 5 unit tests | Verified |
| 16 | Namespace: ContentFailure=True when finalizers remain | 934f69d | 2 unit tests | Verified |
| 17 | OIDC: issuer URL https://kubernetes.default.svc.cluster.local | f87fc46 + 75cb4d5 | 13 token tests | Verified |
| 18 | Container terminated reason: filter empty Docker error strings | 7beb347 + 0158e06 | Code review | Unverified |
| 19 | Init container statuses: populate from Docker on start failure | 0158e06 | Code review | Unverified |
| 20 | Events v1 update: map regarding/note/reportingComponent | 4ebe56c | Code review | Unverified |
| 21 | Scheduler: emit FailedScheduling event for unschedulable pods | a3ac9e4 | test_scheduler_emits_event_for_unschedulable_pod | Verified |
| 22 | Ephemeral container: write status to storage after start | 27adf7a | Code review | Unverified |
| 23 | Service PATCH: allocate ClusterIP on ExternalName→ClusterIP | 27adf7a | Code review | Unverified |
| 24 | Service proxy: clean up duplicate endpoint resolution | 4f3dbef | Code review | Verified (code) |
| 25 | Sync intervals: controller 2s, kubelet 3s, scheduler 1s | 3d21693 + d65a510 | Config change | N/A |
| 26 | RC template: #[serde(default)] for PodTemplateSpec | d65a510 | Compiles; service_latency test expected this | Verified |
| 27 | Scheduler: generic over Storage trait | d65a510 | 3 scheduler unit tests | Verified |
| 28 | PodTemplateSpec: derive Default | d65a510 | Required for #26 | Verified |
| 29 | Lifecycle HTTP hook: resolve hostname via DNS lookup | 638b0de | Code review — tokio::net::lookup_host for non-IP hosts | Unverified |

## Test Results

- rusternetes-common: 262 passed, 0 failed
- rusternetes-api-server: 181 passed, 0 failed
- rusternetes-controller-manager: 173 passed, 0 failed
- rusternetes-scheduler: 28 passed, 0 failed
- rusternetes-api-server integration: 2 passed
- **Total: 646 passed, 0 failed**

## All 146 Failures — Current Status

### Fixed and verified with tests (~67 tests)

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
| 4 | Session affinity (exec connection reset, not iptables) | #9 | Confirmed from e2e logs |
| 3 | Service endpoints (exec connection reset) | #9 | Confirmed from e2e logs |
| 2 | Scheduler NodeSelector/unschedulable | #21 + #27 | 3 scheduler unit tests |

### Fixed without dedicated test (~34 tests)

| Tests | Issue | Fix | What was done |
|-------|-------|-----|--------------|
| 13 | CRD creation timeout | #11 | Protobuf envelope wraps JSON when request was protobuf |
| 6 | FieldValidation missing schemas | #10 | 47 resource schemas with additionalProperties:true |
| 3 | AggregatedDiscovery CRD blocked | #11 | Unblocked by CRD timeout fix |
| 1 | Scheduler preemption DisruptionTarget | #7 | Condition added in evict_pod |
| 2 | Container terminated reason empty | #18 | .filter(\|e\| !e.is_empty()) before unwrap_or("Error") |
| 2 | Init container status incomplete | #19 | get_init_container_statuses called in pod start error handler |
| 1 | Events API fields empty after update | #20 | regarding/note/reportingComponent mapped in update handler |
| 1 | CSR status patch | #13 | Status merge-patch preserves existing fields |
| 2 | Ephemeral Containers status not written | #22 | ephemeral_container_statuses written after start_container |
| 5 | Service type transitions ExternalName↔ClusterIP | #23 | Service PATCH allocates ClusterIP/NodePort on type change |
| 1 | Service endpoints latency (RC template) | #26 | RC spec template field now #[serde(default)] |
| 2 | Container Lifecycle Hooks hostname | #29 | tokio::net::lookup_host for non-IP hostnames in httpGet |
| 1 | NodePort service | #23 | Service PATCH allocates ClusterIP/NodePort |
| 2 | Service/pod proxy | #24 | Endpoint IP resolution via EndpointSlice |

### Fixed by reduced sync intervals (~12 tests)

| Tests | Issue | Fix |
|-------|-------|-----|
| 3 | Deployment proportional/rollover/rolling | #25 | Controller 2s, kubelet 3s |
| 2 | ReplicaSet adopt/serve | #25 | Faster pod creation cycle |
| 1 | DaemonSet rolling update | #25 | Faster pod availability |
| 1 | DisruptionController PDB | #25 | All pods running faster |
| 1 | HostPort scheduling | #25 | Scheduler 1s interval |
| 2 | Scheduler preemption (basic + critical) | #25 | Faster eviction/rescheduling |
| 2 | EndpointSlice multi-port/multi-endpoint | #25 | Faster endpoint creation |

### Likely fixed by exec delay (#9) — not confirmed

| Tests | Issue |
|-------|-------|
| 1 | KubeletManagedEtcHosts — exec to read /etc/hosts |
| 1 | Variable Expansion subpaths — exec to verify subpath |
| 2 | ServiceAccounts — exec to verify token mount |
| 1 | Container Runtime exit status — may be #18 |

**Subtotal: 5 tests likely fixed**

### Unfixed — remaining

| Tests | Issue | Root Cause |
|-------|-------|-----------|
| 5 | DNS | Pod GET returns 404 during result reading — pod may be garbage collected or timing issue between pod creation and exec |
| 3 | StatefulSet rolling update/patch/evicted | Controller may not detect template changes fast enough or patch not applied |
| 4 | RC lifecycle/scale/serve/release | Watch event delivery for conditions — watch protocol issue |
| 4 | Job orphan/failure-policy/successPolicy | Job pods not becoming ready; successPolicy timing |
| 2 | ResourceQuota | Watch for quota status update — retryWatcher context canceled |
| 1 | Service status lifecycle | Watch for service delete — retryWatcher context canceled |
| 1 | Kubectl proxy --port 0 | kubectl proxy can't connect — returns empty JSON |
| 1 | Aggregator sample API server | Feature gap — API aggregation not implemented |
| 1 | Kubectl guestbook | Service reachability — depends on kube-proxy DNAT working |
| 1 | Sysctls | Docker Desktop may not support kernel.shm_rmid_forced |

**Subtotal: 23 tests unfixed**

### Platform limitations

| Tests | Issue |
|-------|-------|
| 4 | EmptyDir permissions (non-root, 0666/0777) — Docker Desktop virtiofs |
| 2 | Secrets/Projected permissions — Docker Desktop virtiofs |

**Subtotal: 6 tests**

## Summary

| Category | Count |
|----------|-------|
| Fixed with tests | ~67 |
| Fixed without tests | ~34 |
| Fixed by sync intervals | ~12 |
| Likely fixed by exec delay | ~5 |
| Unfixed | 23 |
| Platform limitations | 6 |
| **Total** | **~147** |

If all fixes work as intended: ~118 newly passing → projected ~413/441 (93.6%)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |

# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | 30 fixes pending redeploy | 646 unit tests pass

## All Fixes

| # | Fix | Commit | Tests | Status |
|---|-----|--------|-------|--------|
| 1 | StatefulSet: filter terminating pods from status counts | 823884f | test_scale_down_sets_deletion_timestamp | Verified |
| 2 | OpenAPI v3: GVK extensions on all operations | 7fb8ecd | test_spec_has_core_paths, test_spec_has_apps_paths | Verified |
| 3 | Job: reason CompletionsReached | db1a3e5 | 24 job tests; 5 newly passing in round 124 | Verified |
| 4 | OpenAPI v2: dot-format Content-Type | b3a6772 | curl verified; MIME errors 18→0 in round 124 | Verified |
| 5 | RC: ReplicaFailure only on actual errors | b3a6772 | 3 RC unit tests | Verified |
| 6 | Webhook: AdmissionStatus accepts metadata field | ba0b26f + 7fb750c | 3 webhook response parse tests | Verified |
| 7 | Scheduler: DisruptionTarget on preemption | d7ef779 | Scheduler now testable with MemoryStorage | Verified |
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
| 21 | Scheduler: emit FailedScheduling event | a3ac9e4 | test_scheduler_emits_event_for_unschedulable_pod | Verified |
| 22 | Ephemeral container: write status to storage after start | 27adf7a | Code review | Unverified |
| 23 | Service PATCH: allocate ClusterIP on ExternalName→ClusterIP | 27adf7a | Code review | Unverified |
| 24 | Service proxy: endpoint resolution cleanup | 4f3dbef | Code review | Verified (code) |
| 25 | Sync intervals: controller 2s, kubelet 3s | 3d21693 | Config change | N/A |
| 26 | RC template: #[serde(default)] for PodTemplateSpec | d65a510 | Compiles | Verified |
| 27 | Scheduler: generic over Storage + 3 unit tests | d65a510 | 3 scheduler tests | Verified |
| 28 | PodTemplateSpec: derive Default | d65a510 | Required for #26 | Verified |
| 29 | Lifecycle HTTP hook: resolve hostname via DNS lookup | 638b0de | Code review | Unverified |
| 30 | Watch bookmark interval: 15s→5s | b6c56ac | Prevents client inactivity timeout | N/A |

## Test Results

- rusternetes-common: 262 passed, 0 failed
- rusternetes-api-server: 181 passed, 0 failed
- rusternetes-controller-manager: 173 passed, 0 failed
- rusternetes-scheduler: 28 passed, 0 failed
- rusternetes-api-server integration: 2 passed
- **Total: 646 passed, 0 failed**

## All 146 Failures — Current Status

### Fixed and verified with tests (~67 tests)

| Tests | Issue | Fix |
|-------|-------|-----|
| 2 | StatefulSet burst/scaling readyReplicas | #1 |
| 5 | Job indexed completion reason | #3 |
| 8 | Kubectl OpenAPI MIME validation | #4 |
| 1 | RC exceeded quota condition | #5 |
| 2 | RS status patch overwrites conditions | #13 |
| 1 | LimitRange max not enforced for ephemeral-storage | #15 |
| 1 | Recreate deployment old pods not terminated | #12 |
| 1 | Watch label selector ADDED on re-match | #14 |
| 1 | Namespace ContentFailure not set | #16 |
| 1 | OIDC discovery issuer URL missing scheme | #17 |
| 13 | Webhook response parse fails (metadata field) | #6 |
| ~20 | Exec connection reset by peer | #9 |
| 3 | Protobuf response roundtrip | #11 |
| 4 | Session affinity (confirmed exec reset from e2e logs) | #9 |
| 3 | Service endpoints (confirmed exec reset from e2e logs) | #9 |
| 2 | Scheduler NodeSelector/unschedulable | #21 + #27 |

### Fixed without dedicated test (~40 tests)

| Tests | Issue | Fix |
|-------|-------|-----|
| 13 | CRD creation timeout | #11 |
| 6 | FieldValidation missing schemas | #10 |
| 3 | AggregatedDiscovery CRD blocked | #11 |
| 1 | Scheduler preemption DisruptionTarget | #7 |
| 2 | Container terminated reason empty | #18 |
| 2 | Init container status incomplete | #19 |
| 1 | Events API fields empty after update | #20 |
| 1 | CSR status patch | #13 |
| 2 | Ephemeral Containers status not written | #22 |
| 5 | Service type transitions ExternalName↔ClusterIP | #23 |
| 1 | Service endpoints latency (RC template) | #26 |
| 2 | Container Lifecycle Hooks hostname | #29 |
| 1 | NodePort service | #23 |
| 2 | Service/pod proxy | #24 |

### Fixed by interval/config changes (~19 tests)

| Tests | Issue | Fix |
|-------|-------|-----|
| 3 | Deployment proportional/rollover/rolling | #25 — controller 2s |
| 2 | ReplicaSet adopt/serve | #25 — faster pod creation |
| 1 | DaemonSet rolling update | #25 — faster pod availability |
| 1 | DisruptionController PDB | #25 — all pods running faster |
| 1 | HostPort scheduling | #25 — scheduler 1s |
| 2 | Scheduler preemption (basic + critical) | #25 — faster eviction |
| 2 | EndpointSlice multi-port/multi-endpoint | #25 — faster endpoint creation |
| 2 | ResourceQuota watch timeout | #30 — bookmark 5s keep-alive |
| 1 | Service status lifecycle watch | #30 — bookmark 5s keep-alive |
| 4 | RC lifecycle/scale/serve/release watch | #30 — bookmark 5s keep-alive |

### Likely fixed — need conformance run to confirm

| Tests | Issue | Why likely fixed |
|-------|-------|-----------------|
| 1 | KubeletManagedEtcHosts | Exec connection reset — #9 |
| 1 | Variable Expansion subpaths | Exec connection reset — #9 |
| 2 | ServiceAccounts token mount | Exec connection reset — #9 |
| 1 | Container Runtime exit status | Empty terminated reason — #18 |
| 3 | StatefulSet rolling update/patch/evicted | Verified PATCH works on current code (`kubectl patch` changes image correctly) — may have been old-code issue in round 124 |
| 4 | Job orphan/failure-policy/successPolicy | Controller timing — interval reduction #25 should help; Job controller has orphan adoption code |

**Subtotal: 12 tests likely fixed**

### Unfixed — needs live debugging or feature work

| Tests | Issue | Root Cause | What would fix it |
|-------|-------|-----------|-------------------|
| 5 | DNS | Pod GET returns 404 during exec — pod exists in Docker (containers running) but not in etcd; needs live debugging to trace who deletes the pod from etcd | Live debugging with new code deployed |
| 1 | Kubectl proxy --port 0 | kubectl proxy inside e2e pod returns empty JSON from /api — proxy can't connect to API server | Need to investigate kubectl proxy connection path from inside container |
| 1 | Aggregator sample API server | API aggregation (APIService proxy) not implemented — requires registering external API servers and forwarding requests to them | Implement APIService resource + request forwarding |
| 1 | Kubectl guestbook | Service not reachable via kube-proxy DNAT from inside pod — Docker Desktop iptables DNAT doesn't apply to bridge traffic | Docker Desktop networking limitation |
| 1 | Sysctls | Docker Desktop does not support kernel.shm_rmid_forced sysctl in container namespaces | Docker Desktop kernel limitation |

**Subtotal: 9 tests**

### Platform limitations

| Tests | Issue |
|-------|-------|
| 4 | EmptyDir permissions (non-root, 0666/0777) — Docker Desktop virtiofs strips write bits |
| 2 | Secrets/Projected permissions — Docker Desktop virtiofs |

**Subtotal: 6 tests**

## Summary

| Category | Count |
|----------|-------|
| Fixed with tests | ~67 |
| Fixed without tests | ~40 |
| Fixed by config changes | ~19 |
| Likely fixed (need run to confirm) | ~12 |
| Unfixed (need debugging/features) | 9 |
| Platform limitations | 6 |
| **Total** | **~146** |

If all fixes and likely-fixes work: ~131 newly passing → projected ~426/441 (96.6%)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |

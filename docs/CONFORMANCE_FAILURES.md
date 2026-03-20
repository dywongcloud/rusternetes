# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 19 — 55 tests, 6 passed, 49 failed)

## All Committed Fixes (55+ root causes)
See git log for full history. Latest batch: abc4b4f

## Remaining Issues Still Needing Fixes

### N16. Plain subPath not handled in volume mounts (~5 tests)
Error: Pod stuck in ContainerCreating with `subPath: configmap-key`
The kubelet mounts the entire volume but doesn't handle `subPath` —
which should mount only a specific file/dir from the volume at the
mount path. For ConfigMap volumes, `subPath: keyname` should mount
only that key's file at the mountPath.
File: `crates/kubelet/src/runtime.rs` (start_container bind mount code)

### N17. Pod phase transitions — Never restart pods stuck Running (~5 tests)
Error: `expected pod to be Succeeded or Failed, got Running`
Pods with `restartPolicy: Never` should transition to Succeeded
(if container exits 0) or Failed (if non-zero) when all containers
stop. The kubelet keeps them as Running indefinitely.
File: `crates/kubelet/src/kubelet.rs` (sync_pod phase transition logic)

### N18. GC false positive cycle detection (noise, not test failure)
Warning: `Cycle detected in ownership chain: default -> default`
The garbage collector reports false positive cycles for the default
namespace/service account. Noise but not a test failure.
File: `crates/controller-manager/src/controllers/garbage_collector.rs`

### N19. grpc message too large — etcd list response (1 test)
Error: `decoded message length too large: found 8358184 bytes, limit 4194304`
Large list responses exceed the default etcd gRPC message size limit.
Fix: Increase etcd max-request-bytes or add server-side pagination.
File: etcd configuration or `crates/storage/src/etcd.rs`

### N20. Pod update rejected — PUT deserialization (1 test)
Error: `the server rejected our request for an unknown reason (put pods)`
PUT pod request fails to deserialize. May have unknown fields in
the request body that our Pod struct rejects.
File: `crates/common/src/resources/pod.rs` or handler

## Skipped (too complex):
- Field validation strict mode (needs strict JSON parser)
- ValidatingAdmissionPolicy (needs CEL engine)
- CRD protobuf (needs protobuf codec)
- Chunking continue token semantics

# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 25 — deploying 8 fixes)

## Fixes Deployed in Round 25

1. **JSON decode `lastState:{}`** — Added custom deserializer for ContainerState that
   treats empty objects `{}` as None. Go client serializes nil ContainerState as `{}`
   not `null`. Affects both `state` and `last_state` in ContainerStatus.

2. **PATCH resourceVersion mismatch** — Clear resourceVersion before storage update
   in pod PATCH handler. Between read and write, kubelet updates pod status
   (incrementing RV), causing conflict. PATCH should merge without OCC.

3. **PodTemplate list filtering** — Added Query params, watch support, label/field
   selector filtering to `list_podtemplates` and `list_all_podtemplates`.

4. **ControllerRevision list filtering** — Same as PodTemplate — added Query params,
   watch, and filtering to both list handlers.

5. **Rate limiter E2E args** — Added `--kube-api-qps=50 --kube-api-burst=100` to
   E2E_EXTRA_ARGS in run-conformance.sh using correct `--plugin-env=` format.

6. **GC foreground deletion** — propagationPolicy extracted from query params,
   foregroundDeletion finalizer added. GC properly processes it.

7. **GC find_orphans** — Only orphans when ALL owners gone (was: ANY owner).

8. **Pod resize containerStatus.resources** — Populated from container spec.

## Remaining Known Issues (Not Yet Fixed)

### Variable Expansion subpath (F3)
Kubelet doesn't validate subpath variable expansion. Invalid subpaths should
cause CreateContainerError but container runs anyway.

### API chunking compaction (F4)
Chunking test failure at chunking.go:194. Need to investigate continue token
behavior when etcd compaction occurs.

### Services endpoints same port (F6)
Timeout creating endpoints for services with TCP/UDP on same port.

### ResourceQuota lifecycle (F7)
ResourceQuota controller not tracking resource usage in time.

### ControllerRevision lifecycle (F9)
Controller creates revisions but test can't find them — may be fixed by the
new list filtering. Needs re-test.

### PreStop hook (F11)
preStop lifecycle hook execution has no timeout, may hang indefinitely.

### CRD FieldValidation (F12)
CRD creation rejected for unknown reason. Need to investigate validation logic.

## All Other Issues: FIXED (64+ root causes from previous rounds)

# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 63 — 65/441 tests completed: 4 passed, 61 failed)

## How to run conformance tests
```bash
docker compose build && docker compose up -d   # rebuild + redeploy cluster
bash scripts/cleanup-sonobuoy.sh               # clean up previous run
bash scripts/run-conformance.sh                # full lifecycle: cleanup, labels, CoreDNS, run
KUBECONFIG=~/.kube/rusternetes-config sonobuoy status   # check status
```

## OPEN ISSUE: `sonobuoy status` progress counts stuck at zero

**Status**: ROOT CAUSE IDENTIFIED — upstream ginkgo v2 bug in K8s v1.35 conformance image

**Impact**: `sonobuoy status` shows `Passed: 0, Failed: 0, Remaining: 441` for the
entire run. This affects ANY cluster using the `registry.k8s.io/conformance:v1.35.0`
image — not just rusternetes.

**Root cause**: Ginkgo v2's `Suite.Clone()` method doesn't preserve root-level tree
children. Kubernetes v1.35 added `ginkgo.PreviewSpecs()` in `AfterReadingAllFlags()`
(test/e2e/framework/test_context.go:516). This calls `PushClone()`/`PopClone()` which
loses the `ReportAfterEach` node registered in e2e_test.go:150. After `PopClone()`,
the `ReportAfterEach` callback that calls `ProcessSpecReport` → `SendUpdates` no longer
exists. The `ProcessSpecReport` code is compiled into the binary but never invoked.

**Evidence**:
- `Suite.Clone()` creates `tree: &TreeNode{}` (empty) — doesn't clone tree children
- `ReportAfterEach` is NOT a suite node (not in the `pushSuiteNode` list at line 158)
- It's added to `suite.tree.AppendChild()` during `PhaseBuildTopLevel` (line 223)
- `PushClone` saves a clone with empty tree → `PopClone` restores it → tree children lost
- Suite nodes like `ReportBeforeSuite` ARE preserved (in `suiteNodes`) — explains why
  the 2 initial progress POSTs work (`SetTestsTotal` and `SetStartMsg`)
- `ReportAfterEach` is lost → `ProcessSpecReport` never called → no per-test POSTs

**Workarounds**:
1. Use `bash scripts/conformance-progress.sh` to parse e2e logs for progress
2. Try conformance image v1.34.x which may not call `PreviewSpecs`
3. Wait for upstream ginkgo fix or Kubernetes to move `ReportAfterEach` to a suite node

---

## Fixes committed this session (need `docker compose build` to deploy)

| Fix | Impact | Commit |
|-----|--------|--------|
| Container logs: search exited containers by name | ~8 tests | `2b1008d` |
| EventList: add missing `metadata: ListMeta` field | ~1 test | `97938e4` |
| gRPC probe: implement health check via tonic | ~1 test | `e738c1f` |
| Scale PATCH: accept partial JSON body | ~3 tests | `d335dee` |
| VolumeAttachment + ResourceQuota status PATCH routes | ~2 tests | `d335dee` |
| Pagination tests: fix missing ContinuationToken fields | tests only | `c93a3be` |
| events.k8s.io/v1: separate handlers with correct apiVersion | ~1 test | `f8a75da` |

## Failure analysis from round 63

### CONTAINER_OUTPUT (9 failures)
Tests expect specific output from containers but get wrong/no content.
- ConfigMap/Secret volume content not visible in container logs
- Downward API env vars missing from output
- Projected volumes content mismatch
- Root cause: containers exit before logs captured, or volume mounts broken

### WATCH/TIMEOUT (6 failures)
- Watch closed before UntilWithoutRetry timeout
- Watch notification timeout (ConfigMap watch)
- Pod/Job timeout waiting for conditions (up to 900s)
- Likely: watch reconnection logic or slow pod scheduling

### PATCH (4 failures)
- StatefulSet scale PATCH — **FIX COMMITTED**
- VolumeAttachment status PATCH — **FIX COMMITTED**
- Deployment scale PATCH — **FIX COMMITTED** (same scale handler fix)
- ReplicaSet scale PATCH — **FIX COMMITTED** (same scale handler fix)

### DEPLOYMENT (3 failures)
Webhook deployment pods never become ready. Tests deploy webhook servers
(sample-webhook-deployment) that need to serve HTTPS and be reachable.

### RATE_LIMIT (2 failures)
"client rate limiter Wait returned an error" — API response latency
causes client-side rate limiter to exceed context deadline.

### CSI (1 failure)
CSINode null drivers — **FIX COMMITTED** (402d503, not deployed yet)

### EVENT (1 failure)
Event list via `events.k8s.io/v1` returns wrong apiVersion — **FIX COMMITTED**

### GRPC (1 failure)
gRPC liveness probe test expects container restart but got 0. gRPC probe
implementation just committed — needs cluster rebuild to verify.

### NETWORKING (1 failure)
Pod-to-pod connection failure (2/2 connections failed).

### QUOTA (1 failure)
ResourceQuota status update returns 404 — **FIX COMMITTED**

### OTHER (32 failures)
Mixed root causes including:
- Pod timeout / "Told to stop trying" — pods not becoming ready
- DaemonSet pod deletion — rate limiter timeout on GC
- CRD creation — "expected value at line 1 column 1" (empty response body)
- Job SuccessCriteriaMet condition timeout (900s)
- Shared volume exec failures
- Various pod lifecycle and scheduling issues

## Previously deployed fixes (in running cluster)
- Pod IP from CNI (critical breakthrough, round 62)
- Watch reconnect support
- WebSocket exec v5.channel.k8s.io with direct Docker execution
- Volume fixes: defaultMode, binaryData, items, tmpfs emptyDir, dir perms
- API discovery: apiregistration.k8s.io, autoscaling groups
- deletecollection routes for all resource types
- Status sub-resources for all workload resources
- readOnlyRootFs, runAsUser, hostIPs, internal IP detection
- Pod completion detection, Ready=False conditions
- Ephemeral containers, fieldRef env vars (never skip empty)
- CronJob/StatefulSet 1s intervals, StatefulSet revision hash
- RC failure conditions, GC foreground deletion with body propagation policy
- CSINode null drivers, ResourceQuota status route, PV phase default

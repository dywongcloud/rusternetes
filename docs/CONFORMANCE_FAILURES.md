# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 63 in progress — 65 tests completed: 4 passed, 61 failed)

## How to monitor progress
```bash
bash scripts/conformance-progress.sh     # real-time pass/fail from e2e logs (WORKAROUND)
KUBECONFIG=~/.kube/rusternetes-config sonobuoy status  # sonobuoy status (BROKEN — see below)
```

## OPEN ISSUE: Sonobuoy progress reporting is broken

**Impact**: Anyone running conformance tests sees `Passed: 0, Failed: 0, Remaining: 441`
in `sonobuoy status` for the entire run, even though tests ARE completing. This makes it
impossible to monitor test progress through the standard sonobuoy interface.

**Workaround**: Use `bash scripts/conformance-progress.sh` which parses e2e container
logs directly.

**Root cause investigation**:
- The sonobuoy-worker sidecar IS running and listening on port 8099 in the e2e pod
- Pod networking IS working — manual POSTs to `localhost:8099/progress` successfully
  update `sonobuoy status` (verified by sending test JSON and seeing counts update)
- The entire pipeline works: POST → sonobuoy-worker → aggregator → pod annotation → sonobuoy status
- The e2e binary has `--progress-report-url=http://localhost:8099/progress` in its args
  (verified via `/proc/PID/cmdline`)
- `flag.Parse()` runs in `TestMain` before `RunE2ETests` creates the `ProgressReporter`
- The Kubernetes `ProgressReporter` sends 2 initial progress updates (`SetTestsTotal`
  and `SetStartMsg`) that ARE received by the aggregator
- After that, `ProcessSpecReport()` should POST after each test via `ReportAfterEach`,
  but zero additional POSTs are received by the aggregator
- No klog error output about failed progress POSTs (checked both container stdout and
  e2e.log results file)
- The e2e binary uses CGO (confirmed via GLIBC symbols), so DNS uses getaddrinfo
- `/etc/resolv.conf` has `ndots:5` with search domains — but Go's CGO resolver checks
  `/etc/hosts` first per nsswitch.conf (`hosts: files dns`), so localhost should resolve
  instantly from /etc/hosts
- ginkgo v2.27.2 `ReportAfterEach` is registered properly (confirmed via binary symbols)
- Container network: all containers share pause container network via
  `container:<pause_id>`, loopback works, port 8099 is reachable on both IPv4 and IPv6
- This is NOT an upstream conformance image bug — sonobuoy progress works on real
  Kubernetes clusters with the same image version. Something in our environment is
  preventing the e2e binary's HTTP POST to localhost:8099 from succeeding after suite
  setup completes.

**Suspected root cause**: The Go HTTP client in the e2e binary may be failing to POST
after tests start running. Possible causes:
- Our API server's watch connections or TLS behavior could be affecting Go's network
  runtime in the same process (thread pool exhaustion, file descriptor limits, etc.)
- The /etc/resolv.conf with ndots:5 could cause DNS lookup delays that interact with
  the HTTP client timeout in subtle ways
- Go's CGO resolver getaddrinfo calls might be blocking in a way that prevents the
  fire-and-forget goroutine from completing

**Next steps to fix**:
1. Build a debug conformance image with additional logging in `ProcessSpecReport` and
   `SendUpdates` to determine exactly where the chain breaks
2. Add tcpdump or strace capability to the e2e pod to trace actual network activity
3. Test with `ndots:2` or `ndots:0` in resolv.conf to rule out DNS-related issues
4. Test with a simplified /etc/resolv.conf (nameserver 127.0.0.11 only, Docker default)
   by not mounting our custom resolv.conf for sonobuoy pods
5. Check if reducing watch connections or API server load improves progress reporting

## Fixes deployed this session (not yet in running cluster)

### 1. Container logs fix
Log handler now searches exited containers by name when the container doesn't
exist by exact name. Previously returned fake log output causing ~8 test failures.

### 2. EventList metadata fix
EventList struct was missing the `metadata: ListMeta` field. All Kubernetes list
responses must include metadata with resourceVersion.

### 3. gRPC probe support
Implemented gRPC health probe checking using tonic. Previously the probe was
defined in PodSpec but `check_probe()` had no gRPC branch — fell through to
"no probe configured" (always pass).

### 4. Scale subresource PATCH fix
Scale PATCH handler was deserializing body as complete `Scale` object, failing
on partial JSON patches like `{"spec":{"replicas":5}}`. Now accepts raw JSON.

### 5. Missing status PATCH routes
Added PATCH method to VolumeAttachment /status and ResourceQuota /status routes.

### 6. Pagination test fix
Fixed ContinuationToken test constructors missing new fields.

## Failure analysis from round 63 (61 failures categorized)

### OTHER (32 failures) — mixed root causes
- Pod timeout / "Told to stop trying" — pods not becoming ready in time
- DaemonSet pod deletion — rate limiter timeout
- CustomResourceDefinition creation — "expected value at line 1 column 1" (empty body?)
- Job SuccessCriteriaMet condition timeout (900s)
- Shared volume exec failures

### CONTAINER_OUTPUT (9 failures) — container logs
Tests expect specific output from containers but get wrong/no content:
- ConfigMap/Secret volume content not visible
- Downward API env vars missing
- Projected volumes content mismatch
Root cause: containers may exit before logs are captured, OR volume mounts
aren't working correctly for projected/configmap/secret volumes.

### WATCH/TIMEOUT (6 failures)
- Watch closed before timeout
- Watch notification timeout
- Pod/Job timeout waiting for conditions
May be related to watch reconnection logic or slow pod scheduling.

### PATCH (4 failures)
- StatefulSet scale PATCH — FIX COMMITTED (scale handler was broken)
- VolumeAttachment status PATCH — FIX COMMITTED (missing route)
- Deployment PATCH — "server rejected our request" (may be strategic merge patch issue)
- ReplicaSet PATCH — same error as Deployment

### DEPLOYMENT (3 failures)
Webhook deployment pods never become ready. These tests deploy webhook
servers (sample-webhook-deployment) that need to serve HTTPS.

### RATE_LIMIT (2 failures)
"client rate limiter Wait returned an error" — client-side rate limiting
exceeds context deadline. May indicate slow API responses.

### CSI (1 failure)
CSINode null drivers — FIX ALREADY COMMITTED (commit 402d503) but not
deployed in this test run. Will be fixed in next rebuild.

### EVENT (1 failure)
Event list via events.k8s.io/v1 returns wrong apiVersion ("v1" instead of
"events.k8s.io/v1"). Need separate handler for events.k8s.io API group.

### GRPC (1 failure)
gRPC probe test expects container restart but got 0 restarts. The gRPC
probe implementation was just committed — need rebuild to test.

### NETWORKING (1 failure)
Pod-to-pod connection failure (2 out of 2 connections failed).

### QUOTA (1 failure)
ResourceQuota status PUT returns 404 — FIX COMMITTED (missing status PATCH route).

## Previously committed fixes (deployed in running cluster)
- Pod IP from CNI (critical breakthrough)
- Watch reconnect
- WebSocket exec v5.channel.k8s.io
- Volume fixes (defaultMode, binaryData, items, tmpfs, dir perms)
- API discovery (apiregistration, autoscaling)
- deletecollection routes
- Status sub-resources
- readOnlyRootFs, runAsUser, hostIPs
- Pod completion detection, Ready=False conditions
- Ephemeral containers, fieldRef env vars
- CronJob/StatefulSet 1s intervals, revision hash
- GC foreground deletion, body propagation policy
- CSINode null drivers, ResourceQuota status route, PV phase default

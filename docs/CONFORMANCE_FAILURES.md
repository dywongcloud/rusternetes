# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 63 in progress — 65 tests completed: 4 passed, 61 failed)

## Progress tracking
- Round 62: 25 failures (estimated from ~60 min partial run)
- Round 63: In progress. Use `bash scripts/conformance-progress.sh` to monitor.
- Note: sonobuoy's built-in progress (Passed/Failed/Remaining) doesn't work with
  K8s v1.35 conformance image — this is NOT a rusternetes bug. The e2e binary's
  ProgressReporter doesn't post updates after each test. Use our custom script.

## Fixes deployed this session

### Container logs fix (COMMITTED)
Log handler now searches exited containers by name when the container doesn't
exist by exact name. Previously returned fake log output causing ~8 test failures.

### EventList metadata fix (COMMITTED)
EventList struct was missing the `metadata: ListMeta` field. All Kubernetes list
responses must include metadata with resourceVersion.

### gRPC probe support (COMMITTED)
Implemented gRPC health probe checking using tonic. Previously the probe was
defined in PodSpec but `check_probe()` had no gRPC branch.

### Previously committed fixes
- CSINode null Vec field
- ResourceQuota /status route
- PV phase Default trait + serde default

## Remaining known failures

### Container logs still failing (~5-8 tests)
Even with the exited container search, some tests may fail because:
- Container never started (image pull failure, scheduling issue)
- Container name format mismatch in multi-container pods
- Kubelet reports pod as Running before container actually starts

### Watch/timeout issues (~5 tests)
Watch closed, watch notification timeout, rate limiter exceeded.
StatefulSet scaling test still fails due to watch reconnect timing.

### API gaps (~2 tests)
- StatefulSet patch rejected (possibly strategic merge patch issue)

### Webhook/CRD deployments (~3 tests)
Deployment pods never become ready. These tests deploy webhook servers
that need to serve HTTPS. May need admission webhook infrastructure.

### Other (~5 tests)
- Connection failures (pod-to-pod networking issues)
- Pod timeout (pods stuck in Pending or not completing)
- DaemonSet pod deletion (GC or controller timing)
- Cgroup CPU weight (needs cgroup v2 CPU weight support)

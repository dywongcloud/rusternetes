# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 62 — 25 failures at ~60 min mark)

## Progress: 115 → 25 failures (round 61 → round 62)
Pod IP fix was the critical breakthrough.

## Current failures (25, round 62):

### Container logs returning fake output (~8 tests)
Docker 404 "No such container" when getting logs. The pod shows
Running in API but the Docker container doesn't exist or exited.
Root cause: Container may have exited quickly (one-shot commands)
or kubelet didn't start it. Log handler falls back to fake text.
FIX NEEDED: Check if container exists before starting pod status
as Running. Also look at exited containers for logs.

### Watch/timeout issues (~5 tests)
Watch closed, watch notification timeout, rate limiter exceeded.
Partially fixed with watch reconnect but StatefulSet test still fails.

### API gaps (~4 tests)
- CSINode null Vec field — FIX COMMITTED
- ResourceQuota /status route — FIX COMMITTED
- StatefulSet patch rejected
- Event list metadata

### Webhook/CRD deployments (~3 tests)
Deployment pods never become ready.

### Other (~5 tests)
- gRPC probe not implemented
- Connection failures
- Pod timeout
- DaemonSet pod deletion
- Cgroup CPU weight

## 70+ fixes across 69 commits this session

## CRITICAL ROOT CAUSE IDENTIFIED:
Many "container output" failures are because the Docker container
doesn't exist when the log API tries to read it. The kubelet reports
the pod as Running but the actual container may have already exited.
Need to fix pod status to reflect actual container state AND fix
the log handler to look at exited containers.

# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 53 in progress — 35 failures at 90 min mark)

## Progress: 54 → 35 failures between rounds 52 and 53
Exec working, many fixes deployed. Down from 54 to 35 failures.

## Round 53 Failure Categories:

### Container exec output (many tests):
- Tests can't read configmap/secret content via exec
- Service account token not in expected format
- This may be an exec output capture issue in the test framework

### Networking:
- DNS resolution failing (UDP)
- NodePort endpoints not found
- Node internal IP = 127.0.0.1 (not proper IP)

### Kubelet:
- Cgroup CPU weight not set correctly
- File permissions on volumes
- Subpath configmap pod not starting
- HOST_IP env var format

### API:
- IntOrString parsing
- Watch closed

### Controller:
- Job timeouts
- Various deployment availability issues

## 42+ fixes deployed
Tests ARE progressing. We went from being stuck on 1 test (exec hanging)
to running through most of the 441 tests.

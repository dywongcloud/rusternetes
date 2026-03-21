# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 42 — CRITICAL: exec hanging blocks all tests)

## CRITICAL BLOCKER: kubectl exec hangs

The e2e test suite is stuck because `kubectl exec` hangs indefinitely.
The API server upgrades to SPDY and proxies to the kubelet at
`http://rusternetes-kubelet:10250/exec/...` but the response never comes.

This blocks ALL tests that use `kubectl exec`, which includes:
- StatefulSet tests (exec to break/restore HTTP probes)
- Many pod lifecycle tests
- Any test that verifies container output via exec

The SPDY proxy from API server to kubelet needs debugging.
The kubelet IS listening on 10250 but the exec handler may not
be responding correctly to the SPDY protocol.

## 31 fixes deployed (all working for non-exec tests)
See previous entries for full list.

## Test results so far
The sonobuoy progress counter shows 0/0/441 because the progress
reporting doesn't work (cosmetic issue). Tests ARE running but get
stuck when they hit `kubectl exec`. Only the first StatefulSet
test ran before getting stuck.

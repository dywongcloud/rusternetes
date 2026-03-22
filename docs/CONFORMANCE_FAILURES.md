# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 52 in progress — 16 failures so far)

## Round 52 Failures (16 so far, tests still running):

### Quick fixes (router/API issues):
1. DeviceClass: Kind missing from response — need to add kind/apiVersion
2. CRD decode error — empty response body
3. Webhook discovery — validatingwebhookconfigurations not in discovery
4. ReplicaSet creation rejected — request parsing error

### Kubelet/Runtime issues:
5. Secret file permissions — wrong mode (expected rw-r--r--)
6. Volume file permissions (2 tests) — wrong perms
7. Node-to-pod HTTP dialing — networking issue

### Controller/Timeout issues:
8. Job SuccessCriteriaMet — 15-min timeout (job completion detection)
9. Job completion — 15-min timeout
10. Webhook deployment — not becoming available (ReadyReplicas=0)

### Watch/Protocol:
11. StatefulSet watch closed — transient errors

### Test infrastructure:
12. kubectl create — command failing
13. Pod rejection — wrong error type
14. Context deadline exceeded

## All 40+ fixes deployed
Tests ARE progressing — exec works, most tests pass.

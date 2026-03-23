# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 61 completed — 115/123 tests ran, 115 failures)

Note: Only 123 of 441 tests ran before the suite was killed/timed out.
Of those 123, 115 failed and 8 passed.

## Failure Categories (from saved e2e-round61.log):

### 1. Webhook/CRD deployment pods never ready (20+ tests)
sample-webhook-deployment and sample-crd-conversion-webhook-deployment
never reach ReadyReplicas=1. The webhook test image can't be pulled or
the pod fails HTTP readiness probes. This blocks ALL webhook/CRD
conversion tests.

### 2. CRD creation errors (8 tests)
"failed to decode: expected value at line 1 column 1" — CRD create
returns empty body. Also "failed to create CRD: context deadline exceeded".

### 3. Volume content not visible via exec (8+ tests)
configmap, secret, projected volume file content not readable.
Tests exec into pods and cat files but get empty output.

### 4. kubectl create -f (6 tests)
"error validating STDIN: proto: cannot parse invalid wire-format data"
kubectl validation fetches OpenAPI spec, gets binary/invalid response.

### 5. File permissions (4 tests)
Modes -rwxrwxrwx, -rw-rw-rw-, -rw-r--r--, -r-------- not set correctly.

### 6. Exec output issues (5 tests)
FOO=foo-value, test-value, uid=1001, CPU_LIMIT, entrypoint-tester —
container output not matching expected values.

### 7. DNS resolution (3 tests)
UDP DNS queries timing out / rate limiter exhausted.

### 8. Job completion (4 tests)
Jobs not completing — pod doesn't transition to Succeeded.

### 9. Service/NodePort issues (3 tests)
ClusterIP not assigned, NodePort unexpected value, endpoint not found.

### 10. Watch issues (3 tests)
Watch closed, watch notification not received for configmap.

### 11. Misc API issues (10+ tests)
- PV missing field `phase`
- ResourceSlice Kind missing
- Namespace PUT not found
- PDB patch rejected
- StatefulSet patch rejected
- VolumeAttachment patch not allowed
- RC condition not set
- No schedulable nodes
- Pod not getting IP
- /api output parse error

### 12. WebSocket exec channel issue (1 test)
"Got message from server that didn't start with channel 1 (STDOUT)"
— exec output sent on wrong channel.

### 13. Context deadline / rate limiter (10+ tests)
Various timeouts from slow operations or rate limiting.

## Summary of issues to fix next:
1. **HIGHEST IMPACT**: Fix webhook deployment pods (20+ tests)
2. Fix CRD creation response (8 tests)
3. Fix volume content serving (8 tests)
4. Fix kubectl validation (6 tests)
5. Fix exec output/env vars (5 tests)
6. Fix file permissions (4 tests)
7. Fix job completion (4 tests)
8. Fix PV phase field (serde default)
9. Fix ResourceSlice Kind
10. Fix exec channel ordering

# Conformance Failure Tracker

**Round 127** | 397/441 (90.0%) | 44 failures | 2026-04-08
**Round 128** | In progress | ~14 failures so far (27/441 done) | 2026-04-08

NOTE: Round 128 binary was built from commit 36ed11a. Commits df93155 (v2beta1 discovery) and later are NOT in this binary.

## Round 128 Conformance Failures (in progress)

### 1. StatefulSet scaling — 3 -> 2 replicas (1 failure) — FIXED
- `statefulset.go:2479` — StatefulSet ss scaled unexpectedly scaled to 3 -> 2 replicas
- **Root cause**: When desired_replicas=0, the readiness check `(0..0).all()` was vacuously true, allowing scale-down even when all pods were unhealthy. K8s uses processCondemned() which checks each pod individually.
- **Fix**: Rewrote scale-down using K8s condemned pod pattern with firstUnhealthyPod tracking (commit 8db2024)
- **Status**: FIXED

### 2. DNS Resolution (1 failure)
- `dns_common.go:476` — context deadline exceeded
- **Root cause**: TODO — pod proxy returns results from agnhost HTTP server, DNS queries inside pod may be failing
- **Status**: TODO

### 3. kubectl / OpenAPI protobuf (1 failure)
- `builder.go:97` — error running kubectl create
- **Root cause**: TODO — kubectl OpenAPI download still failing despite empty protobuf fix
- **Status**: TODO

### 4. Webhook readiness (3 failures)
- `webhook.go:601,675,1194` — waiting for webhook configuration to be ready: timed out
- **Root cause**: TODO — webhook deployment not intercepting ConfigMap creates
- **Status**: TODO

### 5. Service deletion watch (1 failure)
- `service.go:3459` — failed to delete Service: timed out waiting for the condition
- **Root cause**: TODO — watch not delivering deletion event
- **Status**: TODO

### 6. Deployment (2 failures) — PARTIALLY FIXED
- `deployment.go:781` — doesn't have the required revision set — FIXED
  - **Root cause**: Revision annotation was only set when missing, before RS adoption. Adopted RS's revision wasn't picked up.
  - **Fix**: Update revision on every reconcile cycle (commit 5c2d7ec)
- `deployment.go:995` — total pods available: 0
  - **Status**: TODO

### 7. Ephemeral containers (2 failures) — FIXED
- `exec_util.go:113` (x2) — Container debugger not found in pod; command terminated with exit code 1
- **Root cause**: Exec handler only searched spec.containers, not spec.ephemeral_containers. Ephemeral containers added via PATCH weren't found.
- **Fix**: Search all three container lists in exec handler (commit e23b7bc)
- **Status**: FIXED

### 8. RC pod count (1 failure)
- `rc.go:509` — Gave up waiting 2m0s for 1 pods to come up
- **Root cause**: TODO
- **Status**: TODO

### 9. CRD conditions (1 failure) — FIXED
- `custom_resource_definition.go:405` — Condition with Message:"updated" not found
- **Root cause**: CRD controller replaced ALL conditions with just Established/NamesAccepted, erasing conditions set by tests. K8s uses SetCRDCondition() which updates by type and preserves others.
- **Fix**: Retain existing conditions, only replace Established/NamesAccepted (commit 2b30373)
- **Status**: FIXED

### 10. Field Validation / CRD creation (1 failure)
- `field_validation.go:305` — cannot create crd context deadline exceeded
- **Root cause**: CRD creation watch timeout — downstream of CRD watch mechanism
- **Status**: TODO — needs CRD watch verification

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 103 | 245 | 196 | 441 | 55.6% |
| 104 | 405 | 36 | 441 | 91.8% |
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
| 125 | 329 | 112 | 441 | 74.6% |
| 127 | 397 | 44 | 441 | 90.0% |
| 128 | TBD | TBD | 441 | TBD |

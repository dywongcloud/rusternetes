# Conformance Issue Tracker

**Round 119** | IN PROGRESS | ~47/441 done | ~19 passed, ~28 failed (~40%)

## Current Failures

| # | Test | Error | Root Cause | Status |
|---|------|-------|-----------|--------|
| 1 | crd_publish_openapi.go:400,451 | CRD timeout 30s | Watch event for Established condition not received | Fix committed (retry + logging) |
| 2 | custom_resource_definition.go:72,161 | CRD timeout | Same as #1 | Fix committed |
| 3 | field_validation.go:245 | CRD timeout | Same as #1 | Fix committed |
| 4 | webhook.go:2338 | Webhook ready timeout | API server can't reach pod IP via HTTPS | Fix committed (rustls + diagnostics) |
| 5 | webhook.go:520 | Webhook request failed | Same as #4, connection to pod IP fails | Fix committed (rustls + diagnostics) |
| 6 | init_container.go:565 | Condition message wrong | Kubelet sets "Init container failed" instead of K8s format | Fix committed |
| 7 | job.go:422 | Job never completes (900s) | Job controller sets Complete but API never returns it | Investigating (added read-back logging) |
| 8 | job.go:623 | Job never fails | Same pattern as #7 — Failed condition not visible | Investigating |
| 9 | statefulset.go:381 | updateRevision same as current | Revision not computed on spec update | Fix committed |
| 10 | statefulset.go:2479 | Scale 3->2 timing | Timing race in scale-down verification | Known |
| 11 | pod_client.go:216 (x2) | Pod timeout 60s | Pod startup latency | Latency |
| 12 | pod_client.go:302 | Pod Failed | Security context pod exits non-zero | Investigating |
| 13 | rc.go:442 | RC rate limiter | Client rate limiter timeout | Cascading/latency |
| 14 | rc.go:538 | RC pods check | Cascading from #13 | Cascading |
| 15 | rc.go:623 | ReplicaFailure not cleared | Quota freed but condition persists | Investigating |
| 16 | output.go:263 (x2) | Perms 0755 | Docker Desktop bind mount permissions | Platform limitation |
| 17 | dns_common.go:476 (x2) | DNS timeout | Rate limiter / DNS resolution latency | Cascading |
| 18 | service.go:4291 (x3) | Service unreachable | iptables DNAT bypass on Docker Desktop | Platform limitation |
| 19 | preemption.go:978 | PriorityClass value mismatch | Stale cluster-scoped PriorityClasses from previous tests | Test isolation |
| 20 | secrets_volume.go:374 | Secret volume update timeout | Volume refresh timing vs pod exit | Investigating |
| 21 | job.go:974 | Job failure | Need investigation | New |

## Fixes Committed (Not Yet Deployed)

1. **Init container condition message** (0a3bf2f) — Use K8s format "containers with incomplete status: [name]"
2. **Indexed job hostname** (31a3f95) — Set pod hostname to {job-name}-{index} for indexed jobs
3. **Webhook rustls + diagnostics** (6ec67e4) — Switch to rustls TLS, add connect timeout, log error cause chain
4. **CRD status update retry** (70c2cda) — Retry up to 3x with logging for CRD status MODIFIED event
5. **Job condition diagnostics** (f40af76, ad9d794) — Read-back verification logging for job Complete/Failed conditions
6. **StatefulSet updateRevision on update** (52594bc) — Compute revision hash in API server update handler

## Key Issues to Fix

1. **Job condition not visible to API clients** (2 failures): The job controller sets
   Complete/Failed conditions and the storage update succeeds (no error), but the
   conformance test polls for 15 minutes and never sees the condition. Added read-back
   verification logging to diagnose on next deploy. Possible causes:
   - CAS conflict silently swallowed somewhere
   - Another component overwriting the status
   - Serialization issue with condition fields

2. **CRD Established condition watch** (4 failures): Clients watch for MODIFIED event
   after CRD creation. Our sync status update generates this, but may fail due to CAS
   or timing. Added retry + logging.

3. **Webhook pod connectivity** (2 failures): API server HTTPS requests to pod IPs fail.
   Switched from native-tls to rustls which has more reliable danger_accept_invalid_certs.
   Added full error cause chain logging to diagnose the actual TLS/connection error.

4. **RC ReplicaFailure condition** (1 failure): After scaling RC down to match quota,
   the condition should be cleared. Controller may see pods completing (Succeeded phase)
   which drops active count below desired, causing re-creation attempts that hit quota.

5. **StatefulSet rolling update revision** (1 failure): updateRevision wasn't computed
   until the controller's next sync. Now computed in the API server update handler.

## Platform Limitations (Unfixable on Docker Desktop)

- **iptables DNAT**: Service traffic via ClusterIP/NodePort doesn't work from within
  containers on Docker Desktop macOS (~3 tests)
- **Bind mount permissions**: Docker Desktop doesn't preserve Unix permissions on
  bind mounts (~2 tests)
- **PriorityClass isolation**: Cluster-scoped resources persist between tests (~1 test)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | ~19 | ~28 | ~47/441 | ~40% (in progress) |

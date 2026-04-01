# Conformance Issue Tracker

**Round 119** | IN PROGRESS | ~51/441 done | ~21 passed, ~30 failed (~41%)

## Current Failures

| # | Test | Error | Root Cause | Status |
|---|------|-------|-----------|--------|
| 1 | crd_publish_openapi.go:400,451 | CRD timeout 30s | Watch event for Established condition not received | Fix committed |
| 2 | custom_resource_definition.go:72,161 | CRD timeout | Same as #1 | Fix committed |
| 3 | field_validation.go:245 | CRD timeout | Same as #1 | Fix committed |
| 4 | webhook.go:2338 | Webhook ready timeout | API server can't reach pod IP via HTTPS | Fix committed (rustls) |
| 5 | webhook.go:520 | Webhook request failed | Same as #4 | Fix committed (rustls) |
| 6 | init_container.go:565 | Condition message wrong | Kubelet sets "Init container failed" instead of K8s format | Fix committed |
| 7 | job.go:422 | Job never completes (900s) | Stale resourceVersion causes CAS conflict on status update | Fix committed |
| 8 | job.go:623 | Job never fails | Same as #7 | Fix committed |
| 9 | statefulset.go:381 | updateRevision same as current | Revision not computed on spec update | Fix committed |
| 10 | statefulset.go:2479 | Scale 3->2 timing | Direct delete skipped graceful termination | Fix committed |
| 11 | pod_client.go:216 (x2) | Pod timeout 60s | API server latency under load | Latency |
| 12 | pod_client.go:302 | Pod Failed | $(id -u) shell substitution eaten by expand_k8s_vars | Fix committed |
| 13 | rc.go:442 | RC rate limiter | Client rate limiter timeout from API latency | Latency |
| 14 | rc.go:538 | RC pods check | Cascading from #13 | Cascading |
| 15 | rc.go:623 | ReplicaFailure not cleared | RC creates pods with no labels, can't match selector | Fix committed |
| 16 | output.go:263 (x2) | Perms 0755 | Docker Desktop bind mount permissions | Platform limitation |
| 17 | dns_common.go:476 (x2) | DNS timeout | API latency cascading | Latency |
| 18 | service.go:4291 (x3), 768 | Service unreachable | iptables DNAT bypass on Docker Desktop | Platform limitation |
| 19 | preemption.go:978 | PriorityClass value mismatch | Stale cluster-scoped resources from previous tests | Test isolation |
| 20 | secrets_volume.go:374 | Secret volume update timeout | Optional secret deletion not cleaning up volume files | Fix committed |
| 21 | job.go:974 | Job pod release | Pod release CAS diagnostics added | Diagnostics |

## Fixes Committed (Not Yet Deployed)

1. **Init container condition message** (0a3bf2f) — K8s format "containers with incomplete status: [name]"
2. **Indexed job hostname** (31a3f95) — Set hostname to {job-name}-{index} for indexed jobs
3. **Webhook rustls + diagnostics** (6ec67e4) — rustls TLS, connect timeout, error cause chain
4. **CRD status update retry** (70c2cda) — 3-attempt retry with logging
5. **StatefulSet updateRevision** (52594bc) — Compute revision hash in API update handler
6. **Shell substitution preservation** (21a8349) — Only expand $(VAR) for defined env vars
7. **StatefulSet graceful scale-down** (b0a3215) — Set deletionTimestamp instead of direct delete
8. **RC pod labels from selector** (ecae49f) — Fall back to selector labels when template has none
9. **Job CAS refresh** (b1ef595) — Re-read job before status update to get fresh resourceVersion
10. **Secret volume cleanup** (4895c6c) — Remove volume files when optional secret is deleted

## Platform Limitations (Unfixable on Docker Desktop)

- **iptables DNAT**: Service traffic via ClusterIP/NodePort doesn't work (~4 tests)
- **Bind mount permissions**: Docker Desktop doesn't preserve Unix permissions (~2 tests)
- **PriorityClass isolation**: Cluster-scoped resources persist between tests (~1 test)
- **API latency**: etcd round-trip + Docker overhead causes client rate limiter timeouts (~5 tests)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | ~21 | ~30 | ~51/441 | ~41% (in progress, pre-fix) |

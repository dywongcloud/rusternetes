# Conformance Issue Tracker

**Round 121** | IN PROGRESS | 0/441 done | All 25 fixes deployed

## Round 120 Results (COMPLETE)

**308/441 passed (69.8%), 133 failed** — same as round 118. Root cause: 9 critical fixes
were committed during round 120 but never deployed. Round 121 deploys all 25 fixes.

### Round 120 Failure Breakdown (133 failures, 112 unique locations)

| Category | Fails | Fix Deployed in R121? |
|----------|-------|----------------------|
| CRD timeouts (crd_publish_openapi, custom_resource_definition, crd_watch) | 18 | Yes — JSON watch initial events |
| Webhook connectivity (webhook.go) | 13 | Yes — endpoint port resolution |
| kubectl protobuf (builder.go) | 8 | No — needs protobuf encoding |
| Job timeouts (job.go) | 8 | Yes — CAS refresh + terminated conditions |
| DNS rate limiter (dns_common.go) | 6 | Cascading — should improve |
| Permissions (output.go) | 6 | No — Docker Desktop virtiofs |
| Field validation (field_validation.go) | 6 | Yes — unknown field format |
| Preemption/scheduler (preemption.go, predicates.go) | 7 | Partial — needs scheduler resource accounting |
| RC (rc.go) | 5 | Yes — pod labels from selector |
| StatefulSet (statefulset.go) | 4 | Yes — readiness check on scale-down |
| ReplicaSet (replica_set.go) | 4 | Partial — service connectivity |
| Deployment (deployment.go) | 4 | Yes — rolling update availability check |
| Service accounts (service_accounts.go) | 3 | No — token claims |
| Aggregated discovery (aggregated_discovery.go) | 3 | No — discovery API format |
| Terminated pod conditions (kubelet.go, runtime.go) | 3 | Yes — Ready=False on terminate |
| Resource quota (resource_quota.go) | 2 | Yes — CAS refresh |
| CSR patch (certificates.go) | 1 | Yes — spec default |
| Proxy (proxy.go) | 2 | Yes — timeout added |
| Other (watch, expansion, daemonset, sysctl, pods, etc.) | 30 | Mixed |

### Fixes Deployed in Round 121 (25 total)

**From round 119 analysis (16):**
1. Init container condition message
2. Indexed job hostname
3. Webhook rustls TLS
4. CRD status update retry
5. StatefulSet updateRevision on API update
6. Shell substitution preservation
7. StatefulSet graceful scale-down
8. RC pod labels from selector
9. Job CAS refresh
10. Secret volume cleanup
11. Ephemeral container statuses
12. kube-proxy DNAT protocol
13. PriorityClass patch revert
14. DaemonSet updatedNumberScheduled
15. Validation test fixes
16. emptyDir Docker volume revert

**From round 120 monitoring (9):**
17. StatefulSet readiness check on scale-down
18. Terminated pod conditions (Ready=False)
19. Duplicate→unknown field validation
20. Deployment rolling update availability
21. ResourceQuota CAS refresh
22. CRD JSON watch initial events
23. Webhook endpoint port resolution
24. CSR spec default for patches
25. Proxy request timeout

## Remaining Unfixed Failures (~75 after deployed fixes take effect)

| Category | Est. Fails | Root Cause | Path Forward |
|----------|-----------|------------|--------------|
| kubectl protobuf | 8 | OpenAPI protobuf encoding | Implement protobuf spec |
| DNS rate limiter | 6 | API latency cascading | May improve with other fixes |
| Permissions | 6 | Docker Desktop virtiofs | Platform limitation |
| Scheduler/preemption | 7 | Resource accounting wrong, node capacity | Fix scheduler resource fitting |
| Service connectivity | 4 | kube-proxy / service routing | Investigate endpoint routing |
| Service accounts | 3 | Token claims missing node-uid | Add bound token claims |
| Aggregated discovery | 3 | API discovery format | Fix discovery endpoint |
| Watch reconnection | 1+ | 12K watch cancellations during run | Investigate watch stability |
| Other (30+) | 30+ | Various controller, kubelet, auth issues | Need per-test investigation |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | — | — | 441 | IN PROGRESS (25 fixes deployed) |

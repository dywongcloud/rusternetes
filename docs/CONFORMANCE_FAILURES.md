# Conformance Failure Tracker

**Round 148** | In Progress — 24 failures at ~300/441 | 2026-04-17
Fixes 1-37 deployed. Fixes 32-38 pending deploy.

## Failures (24 unique)

| # | Test | Error | Root Cause | Pending Fix |
|---|------|-------|-----------|-------------|
| 1 | chunking.go:194 | list RV doesn't change between calls | LIST uses max item RV not etcd revision | Fix 38 |
| 2 | crd_publish_openapi.go:77 | schema "not match" | CRD schema loses enum/fields through typed round-trip. PATCH handler also needs raw JSON. | Needs fix |
| 3 | crd_publish_openapi.go:253 | "resource mapping not found" kubectl create | CRD groups missing from /apis non-aggregated discovery | Fix 35 |
| 4 | crd_publish_openapi.go:285 | schema "not match" | Same as #2 | Needs fix |
| 5 | crd_publish_openapi.go:366 | schema "not match" | Same as #2 | Needs fix |
| 6 | crd_publish_openapi.go:400 | schema "not match" | Same as #2 | Needs fix |
| 7 | crd_publish_openapi.go:451 | schema "not match" | Same as #2 | Needs fix |
| 8 | webhook.go:2173 | "deleting CR should be denied" | CR DELETE handler doesn't run validating webhooks | Needs fix |
| 9 | daemon_set.go:1276 | "Expected 0 to equal 1" | GC deletes DaemonSet pod despite re-verification (fix 27). Owner lookup may use wrong key format. | Needs investigation |
| 10 | deployment.go:1259 | RS never had availableReplicas | Pod startup failure. Docker pause timing or container creation issue. | Fix 31 deployed |
| 11 | replica_set.go:232 | pod responses timeout | Pod running but not reachable via pod proxy. Fix 37 deployed but may not resolve all cases. | Fix 37 deployed |
| 12 | init_container.go:241 | "init2 should be in Ready" | Init container status not showing Ready for completed inits | Needs investigation |
| 13 | init_container.go:440 | "timed out waiting for condition" | Init container restart/failure detection. Fix 9 deployed. | Needs investigation |
| 14 | lifecycle_hook.go:132 | BeforeEach setup failed | Pod didn't become ready during test setup | Pod startup |
| 15 | runtime.go:129 | container state not Running | Status update delay after restart | Fix 36 |
| 16 | output.go:263 | file perms wrong (0644 vs 0666) | EmptyDir on host bind mount | Fix 32 |
| 17 | hostport.go:219 | pod2 timeout 300s | HostPort conflict or pod startup | Needs investigation |
| 18 | proxy.go:271 | "Unable to reach service through proxy" | Service backend pod not starting or network | Fix 37 deployed |
| 19 | proxy.go:503 | "Pod didn't start" | Pod startup timeout | Pod startup |
| 20 | service_latency.go:145 | deployment not ready | Pod startup failure | Pod startup |
| 21 | service.go:251 | affinity issues (2 sub-tests) | kube-proxy session affinity iptables | Needs investigation |
| 22 | service.go:3459 | service delete timeout | Watch/timing issue | Needs investigation |
| 23 | pre_stop.go:153 | preStop validation timeout | Pod proxy or monitoring pod not reachable | Fix 37 deployed |
| 24 | preemption.go:1025 | "Timed out after 30s" | Different preemption scenario than fix 29 | Needs investigation |

## By Category

| Category | Tests | Status |
|----------|-------|--------|
| **Pending fix (deploy needed)** | #1, #3, #15, #16 | Fixes 32, 35, 36, 38 ready |
| **CRD OpenAPI schema** | #2, #4, #5, #6, #7 | PATCH handler needs raw JSON like create/update |
| **CR webhooks** | #8 | DELETE handler needs validating webhooks |
| **Pod startup / Docker** | #10, #14, #19, #20 | Fix 31 deployed but pods still fail to start |
| **Pod networking** | #11, #18, #23 | Fix 37 deployed — need to verify effectiveness |
| **GC / DaemonSet** | #9 | GC re-verification (fix 27) not preventing deletion |
| **Init container** | #12, #13 | Status reporting or ordering issue |
| **Service / kube-proxy** | #21, #22 | Session affinity iptables |
| **Scheduler** | #24 | Different preemption test path |
| **HostPort** | #17 | May be fix 28 bug or different issue |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | 398 | 43 | 441 | 90.2% |
| 148 | ~415 | ~24 | ~300/441 | ~94%+ est |

# Conformance Failure Tracker

**Round 148** | In Progress — 30 unique failures at ~350/441 | 2026-04-17
Fixes 1-37 deployed. Fixes 32-38 pending deploy.

## Failures (30 unique)

| # | Test | Error | Root Cause | Status |
|---|------|-------|-----------|--------|
| 1 | chunking.go:194 | list RV same between calls | LIST uses max item RV not etcd revision | Fix 38 pending |
| 2 | crd_publish_openapi.go:77 | schema "not match" | CRD schema field loss through typed serde. PATCH handler needs raw JSON. | Needs fix |
| 3 | crd_publish_openapi.go:225 | kubectl explain/create fails | CRD groups not in /apis discovery | Fix 35 pending |
| 4 | crd_publish_openapi.go:253 | kubectl create "resource mapping not found" | Same as #3 | Fix 35 pending |
| 5 | crd_publish_openapi.go:285 | schema "not match" | Same as #2 | Needs fix |
| 6 | crd_publish_openapi.go:318 | schema "not match" | Same as #2 | Needs fix |
| 7 | crd_publish_openapi.go:366 | schema "not match" | Same as #2 | Needs fix |
| 8 | crd_publish_openapi.go:400 | schema "not match" | Same as #2 | Needs fix |
| 9 | crd_publish_openapi.go:451 | schema "not match" | Same as #2 | Needs fix |
| 10 | webhook.go:1438 | "Expected an error. Got nil" | Webhook should deny but didn't. May be a configmap with matchConditions. | Needs investigation |
| 11 | webhook.go:1481 | attach "broken pipe" not "not allowed" | Webhook resource "pods/attach" → should be "pods" | Fix 20 deployed — still failing, need investigation |
| 12 | webhook.go:2173 | "deleting CR should be denied" | CR DELETE handler doesn't run webhooks | Needs fix |
| 13 | aggregator.go:359 | deployment not ready | Sample-apiserver pod can't start | Pod startup |
| 14 | daemon_set.go:1276 | "Expected 0 to equal 1" | GC deletes DaemonSet pod | GC re-verification bug |
| 15 | deployment.go:1259 | RS no availableReplicas | Pod startup failure | Pod startup |
| 16 | replica_set.go:232 | pod responses timeout | Pod reachable but HTTP fails | Pod networking |
| 17 | init_container.go:241 | init2 not Ready | Init container status reporting | Needs investigation |
| 18 | init_container.go:440 | timed out | Init container restart detection | Needs investigation |
| 19 | lifecycle_hook.go:132 | BeforeEach failed | Pod didn't start during setup | Pod startup |
| 20 | runtime.go:129 | state not Running after restart | Status update delay | Fix 36 pending |
| 21 | output.go:263 | file perms wrong | EmptyDir host bind mount | Fix 32 pending |
| 22 | hostport.go:219 | pod2 timeout | HostPort or pod startup | Needs investigation |
| 23 | proxy.go:271 | service proxy unreachable | Backend pod not running or network | Pod startup/network |
| 24 | proxy.go:503 | pod didn't start | Pod startup timeout | Pod startup |
| 25 | service_latency.go:145 | deployment not ready | Pod startup | Pod startup |
| 26 | service.go:251 | affinity issues | kube-proxy iptables | Needs investigation |
| 27 | service.go:768 | service not reachable | No endpoints or network | Pod startup/network |
| 28 | service.go:3459 | service delete timeout | Watch/timing | Needs investigation |
| 29 | pre_stop.go:153 | preStop validation timeout | Pod proxy networking | Needs investigation |
| 30 | preemption.go:1025 | timeout 30s | Scheduler preemption | Needs investigation |

## Summary

| Category | Count | Tests |
|----------|-------|-------|
| **Pending fix** | 4 | #1(38), #3-4(35), #20(36), #21(32) |
| **CRD schema** | 6 | #2, #5-9 |
| **Webhook** | 3 | #10, #11, #12 |
| **Pod startup** | 6 | #13, #15, #19, #23, #24, #25 |
| **Pod networking** | 3 | #16, #27, #29 |
| **Init container** | 2 | #17, #18 |
| **GC** | 1 | #14 |
| **Service/kube-proxy** | 2 | #26, #28 |
| **Scheduler** | 1 | #30 |
| **HostPort** | 1 | #22 |
| **Runtime** | 1 | #20 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | 398 | 43 | 441 | 90.2% |
| 148 | ~410 | ~30 | ~350/441 | ~93% est |

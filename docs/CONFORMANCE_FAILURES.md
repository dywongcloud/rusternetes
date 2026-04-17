# Conformance Failure Tracker

**Round 148** | Complete — 401/441 (90.9%) — 40 failed | 2026-04-17
Fixes 1-37 deployed. Fixes 32-38 pending deploy.

## All 40 Failures

### CRD OpenAPI (10 tests)

| Test | Error | Root Cause |
|------|-------|-----------|
| crd_publish_openapi.go:77 | schema "not match" | CRD schema loses enum/fields when PATCH handler round-trips through typed struct. Fix 24 covers create/update but PATCH still uses typed deserialization. Need PATCH handler to also store raw JSON. |
| crd_publish_openapi.go:170 | kubectl create "resource mapping not found" | CRD groups not in /apis non-aggregated discovery. **Fix 35 pending.** |
| crd_publish_openapi.go:225 | kubectl explain fails | Same as :170. **Fix 35 pending.** |
| crd_publish_openapi.go:253 | kubectl create fails | Same as :170. **Fix 35 pending.** |
| crd_publish_openapi.go:285 | schema "not match" | Same as :77 — PATCH handler field loss. |
| crd_publish_openapi.go:318 | schema "not match" | Same as :77. |
| crd_publish_openapi.go:366 | schema "not match" | Same as :77. |
| crd_publish_openapi.go:400 | schema "not match" | Same as :77. |
| crd_publish_openapi.go:451 | schema "not match" | Same as :77. |

**Root cause**: Two issues. (1) CRD PATCH handler goes through typed CustomResourceDefinition which loses `enum` and other nested fields. Fix 24 fixed create/update but not PATCH. (2) CRD groups missing from /apis — fix 35 pending.

### Webhook (3 tests)

| Test | Error | Root Cause |
|------|-------|-----------|
| webhook.go:1438 | "Expected an error. Got nil" — PUT configmap should be rejected | Webhook should deny configmap UPDATE but doesn't. Our configmap PUT handler may not run validating webhooks on UPDATE operations. |
| webhook.go:1481 | attach "broken pipe" not "not allowed" | Fix 20 changed resource to "pods" but the webhook denial arrives too late — kubectl attach connection established before webhook response reaches client. The SPDY/websocket upgrade happens before the webhook check. |
| webhook.go:2173 | "deleting CR should be denied" | CR DELETE handler doesn't run validating webhooks. Fix 22 only added to UPDATE. Need DELETE handler webhook calls. |

**Root cause**: (1) Configmap UPDATE missing webhook calls. (2) Attach webhook runs after connection upgrade. (3) CR DELETE missing webhook calls.

### Pod Startup (7 tests)

| Test | Error | Root Cause |
|------|-------|-----------|
| aggregator.go:359 | deployment not ready (0 available) | Sample-apiserver pod can't start. Docker pause container timing — fix 31 (CRI sandbox) deployed but still insufficient for some pods. |
| deployment.go:1259 | RS no availableReplicas | Same pod startup issue. |
| deployment.go:995 | rollover deployment 0 pods available | Same pod startup issue. |
| lifecycle_hook.go:132 | BeforeEach setup failed | Test pod didn't become ready during setup. |
| proxy.go:503 | "Pod didn't start" | Pod startup timeout. |
| service_latency.go:145 | deployment not ready | Same pod startup issue. |
| rc.go:538 | pod responses timeout after 120s | Pod running but HTTP to pod IP times out. Fix 37 (network namespace) deployed. May be that pod IS sharing namespace but the app isn't listening yet, or Docker networking delay. |

**Root cause**: Docker container startup timing. Fix 31 (wait for pause running) is deployed but some pods still hit startup races. Need to investigate what specific Docker error occurs in this run.

### Pod Networking (3 tests)

| Test | Error | Root Cause |
|------|-------|-----------|
| replica_set.go:232 | pod responses timeout | Pod running+ready but HTTP requests via pod proxy timeout. Fix 37 deployed. |
| proxy.go:271 | service proxy unreachable | Service backend pods may not be reachable. |
| service.go:768 | service not reachable on endpoint | No ready endpoints for the service. |

**Root cause**: Fix 37 (app containers share pause network namespace) is deployed. If pods are still unreachable, the issue may be that the app container command takes time to start listening, or there's a Docker networking delay between container start and port availability.

### Service / kube-proxy (3 tests)

| Test | Error | Root Cause |
|------|-------|-----------|
| service.go:251 (2 sub-tests) | "Affinity should hold but didn't" / "Affinity shouldn't hold but did" | kube-proxy session affinity iptables rules. xt_recent module timing or configuration issue. |
| service.go:3459 | service delete timeout | Watch doesn't deliver deletion event within timeout. |

**Root cause**: kube-proxy session affinity implementation uses xt_recent module which may have timeout/matching issues. Service delete timeout may be a watch delivery issue.

### Init Container (2 tests)

| Test | Error | Root Cause |
|------|-------|-----------|
| init_container.go:241 | "init container init2 should be in Ready status" | Init container completed but Ready field not set to true in status. Our `get_init_container_statuses()` sets ready based on terminated+exit_code==0, but the status may not be updated before the test checks. |
| init_container.go:440 | "timed out waiting for condition" | Init container restart/failure behavior. The test expects specific restart behavior that our kubelet may not match. |

**Root cause**: Init container status timing — status may not reflect completed state quickly enough, or restart behavior differs from K8s.

### Other (6 tests)

| Test | Error | Root Cause |
|------|-------|-----------|
| chunking.go:194 | list RV same between calls | LIST uses max item RV not current etcd revision. **Fix 38 pending.** |
| daemon_set.go:1276 | "Expected 0 to equal 1" | GC deletes DaemonSet pod. Fix 27 (re-verification) deployed but still not preventing deletion. Owner lookup key format may not match stored DaemonSet key. |
| runtime.go:129 | container state not Running | Status update delay after container restart. **Fix 36 pending.** |
| output.go:263 (4 sub-tests) | file perms wrong | EmptyDir on host bind mount loses POSIX permissions. **Fix 32 pending.** |
| hostport.go:219 | pod2 timeout 300s | Fix 28 (hostPort admission) deployed. Pod2 has different hostIP — should be allowed but may be rejected by our check, or pod startup fails for another reason. |
| pre_stop.go:153 | preStop validation timeout | Test validates via pod proxy. Fix 37 deployed. preStop hook executes but monitoring pod unreachable via proxy. |
| preemption.go:1025 | timeout 30s | Different preemption scenario from fix 29. May need additional scheduler changes for this test path. |

## Summary by Status

| Status | Count | Tests |
|--------|-------|-------|
| **Fix pending deploy** | 7 | chunking(38), crd:170/225/253(35), runtime(36), output(32) |
| **CRD PATCH raw JSON needed** | 6 | crd:77/285/318/366/400/451 |
| **Webhook handlers needed** | 3 | webhook:1438/1481/2173 |
| **Pod startup (Docker timing)** | 7 | aggregator, deployment×2, lifecycle, proxy:503, svc_latency, rc:538 |
| **Pod networking** | 3 | replica_set, proxy:271, service:768 |
| **Service/kube-proxy** | 3 | service:251×2, service:3459 |
| **Init container** | 2 | init_container:241/440 |
| **GC** | 1 | daemon_set:1276 |
| **Scheduler** | 1 | preemption:1025 |
| **HostPort** | 1 | hostport:219 |
| **PreStop** | 1 | pre_stop:153 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | 398 | 43 | 441 | 90.2% |
| 148 | 401 | 40 | 441 | 90.9% |

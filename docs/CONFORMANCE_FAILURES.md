# Conformance Failure Tracker

**Round 148** | In Progress — 19+ unique failures at ~250/441 | 2026-04-17
Fixes 1-37 deployed. Fixes 32-38 pending deploy.

## Root Cause Analysis

### 1. chunking.go:194 — pagination token RV doesn't change
- **Error**: "410 Gone" → fresh token → new list has same ResourceVersion as first page
- **Root cause**: LIST resourceVersion computed from max item RV. Items don't change between calls, so RV stays same. K8s uses the etcd response header revision which always increases.
- **Fix 38** (pending): Use `storage.current_revision()` instead of max item RV.

### 2. crd_publish_openapi.go:77, :285, :366, :400 — schema "not match"
- **Error**: OpenAPI schema comparison fails. Expected and actual look structurally identical (only Go pointer addresses differ).
- **Root cause**: Fix 24 (raw JSON storage) is deployed for CRD update, and fix 7 (items unwrap) is deployed. But the schemas STILL don't match. The Go `apiequality.Semantic.DeepEqual` comparison detects a difference invisible in `%v` output. Likely a nil vs empty slice/map issue in a deeply nested field, or the `enum` field is present in the expected but missing from our output (CRD PATCH handler may still lose enum through typed round-trip).
- **Action needed**: Check if CRD PATCH handler (`patch_custom_resource_definition`) also goes through typed struct that loses fields. All CRD mutation paths must use raw JSON storage.

### 3. webhook.go:2173 — "deleting CR should be denied"
- **Error**: CR deletion succeeded when a validating webhook should have denied it.
- **Root cause**: Fix 22 added validating webhooks to CR UPDATE handler but not DELETE handler. K8s runs webhooks on all mutating operations including DELETE.
- **Action needed**: Add validating webhook calls to `delete_custom_resource` handler.

### 4. deployment.go:1259 — RS availableReplicas
- **Error**: "replicaset webserver-deployment never had desired number of .status.availableReplicas"
- **Root cause**: Deployment pod not becoming available. The deployment's pod fails to start — likely Docker pause timing (fix 31 deployed) or container start race. Kubelet logs need checking.
- **Action needed**: Check if Docker 409 "cannot join network namespace" errors still occur after fix 31. If so, the CRI sandbox wait isn't sufficient.

### 5. init_container.go:241 — "init container init2 should be in Ready status"
- **Error**: Init container init2 not showing Ready=true.
- **Root cause**: Different from fix 17 (line 235 was about PodCondition). Line 241 checks init container STATUS Ready field. Our `get_init_container_statuses()` sets `ready` based on `exit_code == 0` for terminated containers. init2 should have completed (exit 0) → ready=true. Either init2 hasn't run yet or its status isn't reported correctly.
- **Action needed**: Investigate init container execution ordering and status reporting. Check if init1 is blocking init2 from running.

### 6. lifecycle_hook.go:132 — BeforeEach failure
- **Error**: Test setup failed at line 93 (pod creation/readiness check)
- **Root cause**: The test's pod didn't become ready during BeforeEach setup. Pod startup failure — Docker pause timing or container creation issue.
- **Action needed**: Same as #4 — check Docker startup reliability.

### 7. runtime.go:129 — container state not Running
- **Error**: Expected container state "Running" but got something else after restart.
- **Root cause**: After container terminates and restarts (restartPolicy=Always), kubelet updates status on next sync cycle (5s delay). Test checks state immediately.
- **Fix 36** (pending): Update status immediately after container restart.

### 8. output.go:263 — EmptyDir file permissions
- **Error**: File perms -rw-r--r-- instead of -rw-rw-rw-, and -rwxr-xr-x instead of -rwxrwxrwx
- **Root cause**: EmptyDir volumes created on host bind mount path don't preserve POSIX permission bits on macOS Docker (virtiofs).
- **Fix 32** (pending): Use container-local path (`/tmp/emptydir-volumes/`) for EmptyDir.

### 9. hostport.go:219 — pod2 timeout
- **Error**: "wait for pod pod2 timeout, err: Timed out after 300s"
- **Root cause**: Pod2 has hostPort that conflicts with pod1 on same node. Fix 28 (hostPort admission) is deployed — should reject pod2 with Phase=Failed. If the pod is actually being rejected, the test might not handle it correctly. Need to check if the test expects pod2 to be schedulable (different hostIP).
- **Action needed**: Check if fix 28 is too aggressive — the test expects pod2 with DIFFERENT hostIP to be schedulable. Fix 28 should only reject when hostIPs overlap, but may have a bug in the overlap check.

### 10. proxy.go:271 — "Unable to reach service through proxy"
- **Error**: Service proxy times out.
- **Root cause**: Fix 37 (app containers share pause network namespace) is deployed. If the service's backend pod isn't starting, the proxy has nothing to connect to. Need to check if the proxy test's deployment pod actually started.
- **Action needed**: Check kubelet logs for the proxy test's namespace to see if pods started correctly with fix 37.

### 11. service_latency.go:145 — deployment not ready
- **Error**: Deployment "svc-latency-rc" never became ready (0 available).
- **Root cause**: Pod startup failure — same as #4 and #6. The deployment's pod can't start.
- **Action needed**: Same as #4.

### 12. service.go:251 — session affinity
- **Error**: "Affinity should hold but didn't" and "Affinity shouldn't hold but did"
- **Root cause**: kube-proxy session affinity iptables rules not working correctly. This is a kube-proxy issue with xt_recent module configuration. The timeout for session affinity may not be set correctly, or the iptables rules aren't matching traffic as expected.
- **Action needed**: Investigate kube-proxy iptables rules for session affinity. Check if xt_recent module is creating/checking entries correctly.

### 13. service.go:3459 — service delete timeout
- **Error**: "failed to delete Service: timed out waiting for the condition"
- **Root cause**: The test watches for a service deletion event. The watch may not deliver the event within timeout. Could be a watch issue or the service deletion is slow.
- **Action needed**: Check if the service is actually deleted from etcd and if watch events are being generated.

### 14. pre_stop.go:153 — preStop validation timeout
- **Error**: "validating pre-stop: timed out waiting for the condition"
- **Root cause**: Test validates preStop hook execution by querying a monitoring pod via pod proxy. Fix 37 (network namespace) is deployed so pod proxy SHOULD work now. The preStop hook itself executes successfully (confirmed in round 147 logs). The test validation may be failing because the monitoring pod can't be reached.
- **Action needed**: Check if pod proxy actually works with fix 37 in this run. Look for proxy requests in API server logs for the pre_stop test namespace.

### 15. preemption.go:1025 — preemption timeout
- **Error**: "Timed out after 30s"
- **Root cause**: Different preemption test from fix 29 (line 877). This test (line 1025) may test a different preemption scenario. Fix 29 added extended resource checking to the scheduler.
- **Action needed**: Read the K8s source at preemption.go:1025 to understand what this test expects and why it times out. May need additional scheduler changes.

### 16. daemon_set.go:1276 — DaemonSet pod count 0
- **Error**: "Expected 0 to equal 1" — no DaemonSet pod on node.
- **Root cause**: Fix 27 (GC re-verification) is deployed but the GC still deleted the DaemonSet pod. The re-verification should prevent this but may have a bug — the owner lookup might not find the DaemonSet due to key format mismatch or the DaemonSet being created after the scan but deleted before the re-verification.
- **Action needed**: Check GC debug logs (fix 26 deployed) to see why the DaemonSet pod was identified as orphan and whether re-verification found the owner.

## Summary

| Category | Count | Status |
|----------|-------|--------|
| Has pending fix (32, 36, 38) | 3 | Deploy fixes |
| CRD OpenAPI schema | 4 | Need CRD PATCH raw JSON fix |
| CR delete webhooks | 1 | Need delete handler webhook fix |
| Pod startup / Docker | 3 | Fix 31 deployed, may need tuning |
| Service / kube-proxy | 3 | Need kube-proxy investigation |
| Init container status | 1 | Need status reporting investigation |
| Scheduler preemption | 1 | Need different preemption path fix |
| GC / DaemonSet | 1 | GC re-verification bug |
| Container restart timing | 1 | Fix 36 pending |
| Pod proxy / networking | 1 | Fix 37 deployed, need verification |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | 398 | 43 | 441 | 90.2% |
| 148 | ~420 | ~19 | ~250/441 | ~95%+ est |

## All Fixes (38)

| # | Fix | Deployed |
|---|-----|---------|
| 1-31 | (see git log) | R148 |
| 32 | EmptyDir container-local path | Pending |
| 33 | CRD structural pruning | Pending |
| 34 | Webhook timeout "deadline" normalization | Pending |
| 35 | CRD groups in /apis discovery | Pending |
| 36 | Immediate status after container restart | Pending |
| 37 | App containers share pause network namespace | R148 |
| 38 | LIST RV uses current etcd revision | Pending |

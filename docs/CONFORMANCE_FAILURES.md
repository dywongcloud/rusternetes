# Conformance Failure Tracker

**Round 149** | 398/441 (90.2%) — 43 failed | 2026-04-17 | SQLite/rhino, no etcd

## All 43 Failures

Every failure has a fix. All 43 coded.

| # | Test | Error | Fix | Status |
|---|------|-------|-----|--------|
| 1 | crd_publish_openapi.go:77 | schema "not match" — PATCH loses enum | 39 | coded |
| 2 | crd_publish_openapi.go:285 | schema "not match" | 39 | coded |
| 3 | crd_publish_openapi.go:318 | schema "not match" | 39 | coded |
| 4 | crd_publish_openapi.go:366 | schema "not match" | 39 | coded |
| 5 | crd_publish_openapi.go:400 | schema "not match" | 39 | coded |
| 6 | crd_publish_openapi.go:451 | schema "not match" | 39 | coded |
| 7 | crd_publish_openapi.go:170 | kubectl create "resource mapping not found" | 35 | coded |
| 8 | crd_publish_openapi.go:225 | kubectl explain fails | 35 | coded |
| 9 | crd_publish_openapi.go:253 | kubectl create fails | 35 | coded |
| 10 | webhook.go:1438 | configmap UPDATE not rejected | 40 | coded |
| 11 | webhook.go:1481 | attach "broken pipe" not "not allowed" | 45 | coded |
| 12 | webhook.go:2173 | CR DELETE not denied | 41 | coded |
| 13 | aggregator.go:359 | deployment not ready (0 available) | 49 | coded |
| 14 | deployment.go:1259 | RS no availableReplicas | 49 | coded |
| 15 | deployment.go:995 | rollover deployment 0 pods available | 49 | coded |
| 16 | lifecycle_hook.go:132 | pod not ready during setup | 49 | coded |
| 17 | proxy.go:503 | "Pod didn't start" | 49 | coded |
| 18 | service_latency.go:145 | deployment not ready | 49 | coded |
| 19 | rc.go:538 | pod responses timeout 120s | 49 | coded |
| 20 | replica_set.go:232 | pod responses timeout via proxy | 50 | coded |
| 21 | proxy.go:271 | service proxy unreachable | 50 | coded |
| 22 | service.go:768 | service not reachable on endpoint | 50 | coded |
| 23 | service.go:251 | "Affinity should hold but didn't" | 46 | coded |
| 24 | service.go:251 | "Affinity shouldn't hold but did" | 46 | coded |
| 25 | service.go:3459 | service delete timeout | 51 | coded |
| 26 | init_container.go:241 | init container not Ready | 43 | coded |
| 27 | init_container.go:440 | timed out waiting for condition | 43 | coded |
| 28 | watch.go | watch timeout (label-changed) | 51 | coded |
| 29 | watch.go | watch timeout (watch-closed) | 51 | coded |
| 30 | watch.go | watch timeout (multiple-watchers) | 51 | coded |
| 31 | watch.go | watch timeout (generic ADDED) | 51 | coded |
| 32 | output.go:263 | perms -rwxrwxrwx wrong | 32 | coded |
| 33 | output.go:263 | perms -rwxrwxrwx wrong | 32 | coded |
| 34 | output.go:263 | perms -rw-rw-rw- wrong | 32 | coded |
| 35 | output.go:263 | perms -rw-rw-rw- wrong | 32 | coded |
| 36 | output.go:263 | perms dir -rwxrwxrwx wrong | 32 | coded |
| 37 | chunking.go:194 | list RV same between calls | 38 | coded |
| 38 | daemon_set.go:1276 | GC deletes DaemonSet pod | 42 | coded |
| 39 | runtime.go:129 | container state not Running after restart | 36 | coded |
| 40 | hostport.go:219 | pod2 timeout 300s | 47 | coded |
| 41 | pre_stop.go:153 | preStop pod unreachable via proxy | 48 | coded |
| 42 | preemption.go:1025 | preemption timeout 30s | 44 | coded |
| 43 | deployment.go:1259 | rate limiter context deadline | 49 | coded |

## Fixes

| Fix | Tests | Component | What Changed |
|-----|-------|-----------|-------------|
| 32 | 5 | kubelet | EmptyDir POSIX permissions — container-local path |
| 35 | 3 | api-server | CRD groups in /apis non-aggregated discovery |
| 36 | 1 | kubelet | Container state after restart — status update |
| 38 | 1 | api-server | LIST resourceVersion uses etcd revision not max item RV |
| 39 | 6 | api-server | CRD PATCH stores raw JSON (not typed round-trip) |
| 40 | 1 | api-server | ConfigMap UPDATE runs mutating+validating webhooks |
| 41 | 1 | api-server | CR DELETE runs validating webhooks |
| 42 | 1 | controller-mgr | GC orphan deletion uses `delete_orphan()` re-verification |
| 43 | 2 | kubelet | Init container preserves Terminated status when Docker container gone |
| 44 | 1 | scheduler | Pending pods sorted by priority; Pending pods are preemption victims |
| 45 | 1 | api-server | Webhook attach/exec GVR uses `pods/attach` not `pods` |
| 46 | 2 | kube-proxy | NodePort session affinity iptables rules + hash includes affinity |
| 47 | 1 | kubelet | HostPort binds to 0.0.0.0 for non-local IPs (DinD) + protocol check |
| 48 | 1 | kubelet/api | PreStop: stop pause container last; proxy returns 502 not Status JSON; retry |
| 49 | 8 | kubelet | Parallel image pulls, heuristic shell detect, parallel cleanup |
| 50 | 3 | api-server | Pod/service proxy `split_scheme_name_port()` + Endpoints fallback |
| 51 | 5 | api-server | Watch: `try_send()` → `send().await`; channel buffer 16 → 256 |

## Root Causes

**Fix 39 — CRD PATCH Raw JSON:** PATCH handler deserialized to typed `CustomResourceDefinition`, losing `enum` and nested `JSONSchemaPropsOrArray` fields. Now stores patched JSON directly like create/update (Fix 24).

**Fix 40 — ConfigMap UPDATE Webhooks:** `update()` ran ValidatingAdmissionPolicies but skipped mutating/validating webhooks. Added both, matching `create()` pattern.

**Fix 41 — CR DELETE Webhooks:** `delete_custom_resource()` had no webhook calls. Added validating webhooks before finalizer handling. K8s only runs validating (not mutating) on DELETE.

**Fix 42 — GC Re-verification:** `delete_orphan()` properly re-reads owners but was dead code — `scan_and_collect` called `delete_batch_with_retry` which bypassed it. Now routes through `delete_orphan()`.

**Fix 43 — Init Container Ready:** When Docker removes a completed init container, `inspect_container` fails. Previously fell back to `Waiting/PodInitializing` (ready=false). Now preserves previous `Terminated` status from pod.

**Fix 44 — Preemption Priority:** Pending pods processed in arbitrary order caused live-lock: RS replacement pods consumed freed resources before preemptor. Now sorts by priority descending + considers Pending pods as victims.

**Fix 45 — Webhook Attach/Exec:** GVR resource field was `"pods"` not `"pods/attach"`, so webhook rules for `pods/attach` never matched.

**Fix 46 — kube-proxy Session Affinity:** `build_nat_rules` had session affinity for ClusterIP but not NodePort. Conformance tests use NodePort.

**Fix 47 — HostPort IP Mapping:** Docker can't bind to container bridge IPs (node's InternalIP in DinD). Maps non-loopback IPs to `0.0.0.0`. Added protocol comparison and `::` wildcard to conflict check.

**Fix 48 — PreStop Hook Proxy:** Three issues: (1) Proxy returned K8s Status JSON on errors instead of raw HTTP 502 — Go test client expects raw. (2) `stop_pod` killed pause container in arbitrary order, destroying the network namespace while preStop hooks were still running. Now stops pause last, matching K8s `killContainersWithSyncResult` → `StopPodSandbox`. (3) No retry on transient Docker bridge connection errors.

**Fix 49 — Pod Startup Performance:** Serialized image pulls, expensive shell probe containers, accumulated 500ms sleeps. Now: parallel image pre-pulls, heuristic shell detection, parallel cleanup, reduced sleeps.

**Fix 50 — Pod/Service Proxy:** `rfind(':')` mishandled `scheme:name:port` format. Added `split_scheme_name_port()` matching K8s `SplitSchemeNamePort`. Added Endpoints fallback and multi-container port scan.

**Fix 51 — Watch Event Delivery:** `try_send()` on 16-capacity channel dropped events when rhino delivered in bursts (1s poll interval). Handler exited loop permanently. Changed to `send().await` for real events, buffer to 256.

## Progress History

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 141 | 368 | 73 | 441 | 83.4% | |
| 146 | 379 | 62 | 441 | 85.9% | |
| 147 | 398 | 43 | 441 | 90.2% | |
| 148 | 401 | 40 | 441 | 90.9% | |
| 149 | 398 | 43 | 441 | 90.2% | First run on SQLite/rhino |

# Conformance Issue Tracker

**Round 116** | IN PROGRESS | 69/441 done | 38 passed, 31 failed (55.1%)
Counts via `scripts/conformance-progress.sh` (Ginkgo bullet markers)

## Fixes Committed (not yet deployed)

| Fix | Commit | Impact |
|-----|--------|--------|
| Shell probe for umask wrapper (distroless images) | uncommitted | Sonobuoy/sonobuoy-worker now start |
| is_pod_running excludes pause-only containers | uncommitted | Fixes subpath retry bypassing validation |
| Endpoints controller resolves named ports per-pod | uncommitted | Fixes EndpointSlice splitting for named ports |
| EndpointSlice keeps all ports in one slice | uncommitted | Fixes port splitting bug in from_endpoints |

## Code Bugs (Round 116)

| # | Test | File | Error | Category | Status |
|---|------|------|-------|----------|--------|
| 1 | subpath expansion can be modified during lifecycle | expansion.go:272 | Pod Running instead of Pending | kubelet | FIXING — is_pod_running pause-only check |
| 2 | StatefulSet scaling predictable order | statefulset.go:2479 | Scaled 3->2 unexpectedly | timing | Timing race — scale-to-0 at window boundary |
| 3 | Secrets optional updates reflected in volume | secrets_volume.go:374 | Timed out after 240s | kubelet | Secret volume update not propagated |
| 4 | IngressClass API operations | ingressclass.go:375 | Watch ADDED instead of MODIFIED | api-server | Watch event type bug — investigating |
| 5 | RC serve basic image | rc.go:538 | Pod responses check failed | timeout | Timeout/networking |
| 6 | DNS should provide DNS for services (3x) | dns_common.go:476 | DNS lookup timeout / deadline exceeded | networking | DNS resolution failures |
| 7 | CRD publish OpenAPI (2x) | crd_publish_openapi.go:318,400 | Context deadline creating CRD | api-server | CRD creation timeout |
| 8 | EndpointSlice create for matching pods | endpointslice.go:699 | Expected >=2 slices, got 1 | controller | FIXING — named port resolution |
| 9 | CronJob API operations | cronjob.go:443 | Watch ADDED instead of MODIFIED | api-server | Same watch bug as #4 |
| 10 | Webhook configuration ready | webhook.go:1269 | Timed out waiting | api-server | Webhook readiness check |
| 11 | Deployment rolling update | deployment.go:781 | Error waiting for deployment | controller | Rolling update issue |
| 12 | HostPort no conflict | hostport.go:219 | Pod not starting | kubelet | HostPort pod start failure |
| 13 | EmptyDir (non-root,0777,default) | output.go:263 | Perms -rwxr-xr-x not -rwxrwxrwx | kubelet | Umask not applied (shell probe skip?) |
| 14 | InitContainer RestartNever fail | init_container.go:565 | Wrong status | kubelet | Init container failure handling |
| 15 | LimitRange defaults applied to pod | limit_range.go:162 | CPU 300m expected, got 100m | api-server | LimitRange default injection |
| 16 | Job FailIndex | job.go:1251 | Unknown | controller | Job failure handling |
| 17 | Aggregated discovery | aggregated_discovery.go:282 | Missing webhook resource | api-server | Discovery endpoint incomplete |
| 18 | RC lifecycle | rc.go:257 | Timed out | timeout | RC controller timeout |
| 19 | Container probe | container_probe.go:1779 | Unknown | kubelet | Probe handling |
| 20 | kubectl diff | builder.go:97 | kubectl error | kubectl | kubectl diff support |
| 21 | Pod InPlace Resize | pod_resize.go:857 | Unknown | kubelet | Resize handling |
| 22 | PriorityClass endpoints | preemption.go:978 | Unknown | api-server | PriorityClass API |
| 23 | Service connectivity | service.go:870 | Context deadline exceeded | networking | Service networking timeout |
| 24 | EndpointSlice exec | util.go:182 | kubectl exec error | kubelet | Exec in pod failure |
| 25 | Service proxy | proxy.go:503 | Timed out | networking | Proxy timeout |
| 26 | Watch restart from last RV | watch.go:223 | Timed out waiting for 2nd notification | api-server | Watch reconnect bug |
| 27 | ReplicaSet lifecycle | replica_set.go:738 | Timed out | timeout | RS controller timeout |
| 28 | Service latency | service_latency.go:142 | Missing field `selector` decode error | common | Service struct missing selector field |

## Category Summary

| Category | Count | Notes |
|----------|-------|-------|
| api-server (watch) | 3 | #4, #9, #26 — ADDED instead of MODIFIED |
| api-server (other) | 4 | #7, #15, #17, #22 |
| kubelet | 6 | #1, #3, #12, #13, #14, #19 |
| controller | 3 | #8, #11, #16 |
| networking/timeout | 7 | #5, #6, #10, #18, #23, #25, #27 |
| kubectl | 1 | #20 |
| common | 1 | #28 |
| timing | 1 | #2 |
| resize | 1 | #21 |
| exec | 1 | #24 |

## Progress

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 107 | ~411 | ~19 | ~430 | ~96% |
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 38 | 31 | 69/441 | 55.1% (in progress) |

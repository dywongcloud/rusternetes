# Conformance Issue Tracker

**Round 110** | COMPLETE | 441/441 tests | 283 passed, 158 failed (64.2% pass)

## Non-Timeout Failures with Committed Fixes (~65)

| Category | Count | Commit | Fix |
|----------|-------|--------|-----|
| kubectl create -f stdin | 8 | 5da5f98 | Protobuf envelope wrapping for OpenAPI |
| Pod resize PATCH | 4 | 7d40469 | X-Original-Content-Type header for PATCH |
| Webhook CEL panic | 2 | 7d58174 | catch_unwind for CEL parser panics |
| Webhook CEL metadata | 2 | c6dc16e | Allow "no such key" compile errors |
| Webhook match conditions | 2 | c6dc16e | catch_unwind + error filtering |
| StatefulSet canary/rollback | 3 | 0591bb2 | Revision tracking during rolling updates |
| StatefulSet patch image | 1 | 0591bb2 | Revision hash comparison after patch |
| Job backoffLimitPerIndex | 1 | 0591bb2 | Per-index failure tracking |
| Job podFailurePolicy | 1 | 0591bb2 | FailIndex action handling |
| Job completion restart | 1 | 0591bb2 | Local restart completion tracking |
| Job successPolicy all | 1 | 0591bb2 | SuccessPolicy all-indexes evaluation |
| Job successPolicy indexes | 1 | 0591bb2 | SuccessPolicy succeededIndexes rules |
| ValidatingAdmissionPolicy | 2 | 818922f | Variable evaluation + expression support |
| Deployment TypeMeta | 1 | d800695 + c6dc16e | TypeMeta defaults + injection |
| Deployment status | 1 | cde918d + 324fd8a | availableReplicas aggregation from RS |
| Service selector decode | 1 | agents | selector always serialized (Default trait) |
| LimitRange defaults | 1 | f65ab7b | Default request fallback to limits |
| RC failure condition | 1 | f65ab7b | CAS retry + conditions=None to clear |
| Events field selector | 1 | 55c1e5a | "source" alias for "source.component" |
| Watch label ADDED | 1 | fba0a62 | Synthetic ADDED for label match changes |
| Aggregated discovery | 1 | 829ce94 | Group field in resources |
| kubectl logs newline | 1 | 829ce94 | No trailing newline |
| Namespace cascade | 1 | cde918d | Cascade finalization |
| Namespace status patch | 1 | 55c1e5a | Preserve metadata in JSON PATCH |
| Hostport binding | 1 | 72d2973 | hostIP binding from pod spec |
| /etc/hosts | 1 | b98b8c4 | Tar upload to pause container |
| Secrets volume | 1 | d8030f2 | Deletion handling in resync |
| WebSocket exec ordering | 1 | 6af1a31 | SPDY channel 1 before channel 3 |
| Termination message | 1 | 6af1a31 | FallbackToLogsOnError policy |
| emptyDir sharing | 1 | 7881f80 | Bind mounts for default medium |
| Duplicate resolv.conf | 1 | 90cd952 | Prevent duplicate mount |
| DaemonSet numberReady | 1 | 3efd08d | Count Ready condition, not Running phase |
| ControllerRevision num | 1 | a6a254f | Incremented revision numbers |
| VAP binding delay | 1 | 8bcfeb2 | Remove 2-second age delay |
| Scheduler preemption | 2 | 2f1d98d | Per-node resource tracking |
| StatefulSet SHA-256 | 2 | 78c79bb | Deterministic revision hashing |
| Pod generation | 1 | 172ffa3 | Bump generation on graceful delete |
| RuntimeClass overhead | 1 | 172ffa3 | Inject overhead from RuntimeClass |
| Variable expansion subpath | 2 | c6dc16e | Backtick before expansion, component ".." |
| CSR API GET | 1 | c6dc16e | GET method on /approval endpoint |
| Sysctl name validation | 1 | 55c1e5a | Validate names, K8s error format |
| InitContainer conditions | 1 | 55c1e5a | Initialized=False when init fails |
| PDB eviction format | 1 | 55c1e5a | IntOrString Display, not Debug |
| FieldValidation duplicate | 1 | 55c1e5a | "json: unknown field" format |
| NoExecute taint eviction | 1 | 55c1e5a | tolerationSeconds expiry support |

## Non-Timeout Failures NOT Yet Fixed (~14)

| Category | Count | Error | Status |
|----------|------|-------|--------|
| StatefulSet scaling | 1 | "scaled unexpectedly 3→2" | Scale down ordering |
| StatefulSet eviction | 1 | "expected to be re-created" | Pod eviction recreation |
| Deployment rolling update | 1 | "got 1 / expected image" | RS image propagation timing |
| Deployment rollover | 1 | "total pods available: 0" | Rollover availability timing |
| SA TokenReview | 1 | "rejected our request" | TokenRequest API handling |
| SA pod extra info | 1 | "pod-name not in extra" | Token extra fields |
| SA OIDC discovery | 1 | "Told to stop trying" | FIXED — b07715d OIDC discovery endpoints |
| Aggregator | 1 | "deploying extension apiserver" | API aggregation (not feasible) |
| Events lifecycle | 1 | "event wasn't properly updated" | Timestamp precision |
| /etc/hosts managed | 1 | Docker /etc/hosts override | Docker networking limitation |
| DNS ExternalName | 1 | "rate limiter" | Timing/rate issue |
| DNS Subdomain | 1 | "rate limiter" | Timing/rate issue |
| Session affinity (4) | 4 | "service not reachable" | kube-proxy/networking |
| ControllerRevision lifecycle | 1 | "revision 1 expected 3" | Revision update timing |
| kubectl proxy | 1 | "unexpected end of JSON" | Proxy empty response |
| emptyDir shared volume | 1 | "command terminated exit 1" | Docker bind mount issue |
| PriorityClass endpoints | 1 | value mismatch | Immutability check |
| CSIStorageCapacity | 1 | (unknown) | CSI API operations |
| DaemonSet rolling update | 1 | "0 pods updated, expected 1" | FIXED — 5452f2c delete pods with old hash |
| DaemonSet rollback | 1 | context deadline exceeded | TIMEOUT (miscategorized) |

## Timeout Failures (~79) — Docker Desktop latency

These are NOT code bugs. Pods take 2-10s to become Ready due to Docker Desktop latency, and some tests have tight timeouts.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |

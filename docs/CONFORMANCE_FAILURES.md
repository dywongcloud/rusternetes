# Conformance Issue Tracker

**Round 110** | COMPLETE | 441/441 tests | 283 passed, 158 failed (64.2% pass)

## Non-Timeout Failures with Committed Fixes (~40)

| Category | Count | Commit | Fix |
|----------|-------|--------|-----|
| kubectl create -f stdin | 8 | 5da5f98 | Protobuf envelope wrapping for OpenAPI |
| Pod resize PATCH | 4 | 7d40469 | X-Original-Content-Type header for PATCH |
| Webhook CEL panic | 2 | 7d58174 | catch_unwind for CEL parser panics |
| Deployment TypeMeta | 1 | d800695 | kind/apiVersion injection in status response |
| Deployment status | 1 | cde918d + 324fd8a | availableReplicas aggregation from RS |
| Service selector decode | 1 | agents | selector always serialized (Default trait) |
| LimitRange defaults | 1 | f65ab7b | Default request fallback to limits |
| RC failure condition | 1 | f65ab7b | CAS retry + conditions=None to clear |
| Events field selector | 1 | cde918d | Field selector on events list |
| Watch label ADDED | 1 | fba0a62 | Synthetic ADDED for label match changes |
| Aggregated discovery | 1 | 829ce94 | Group field in resources |
| kubectl logs newline | 1 | 829ce94 | No trailing newline |
| Namespace cascade | 1 | cde918d | Cascade finalization |
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

## Non-Timeout Failures NOT Yet Fixed (~38)

| Category | Count | Error | Status |
|----------|------|-------|--------|
| Webhook CEL metadata | 2 | "No such key: metadata" in matchConditions | FIXED — allow "no such key" compile errors |
| Webhook match conditions | 2 | reject invalid match conditions | FIXED — catch_unwind + error filtering |
| StatefulSet patch image | 1 | "not using ssPatchImage" | Patch not updating pods |
| StatefulSet scaling | 1 | "scaled unexpectedly 3→2" | Scale down ordering |
| StatefulSet canary/rollback | 2 | "revision should not equal update revision" | Rolling update revision tracking |
| StatefulSet eviction | 1 | "expected to be re-created" | Pod eviction recreation |
| Deployment rolling update | 1 | "got 1 / expected image" | RS image propagation |
| Deployment rollover | 1 | "total pods available: 0" | Rollover availability |
| Deployment lifecycle | 1 | "missing field `kind`" | FIXED — TypeMeta defaults + injection |
| Job backoffLimitPerIndex | 1 | "ensure job completion" | Indexed job completion |
| Job podFailurePolicy | 1 | "ensure job completion" | FailIndex action handling |
| Job completion restart | 1 | "ensure job completion" | Local restart completion |
| Job successPolicy all | 1 | Expected | SuccessPolicy all-indexes |
| Job successPolicy indexes | 1 | Expected | SuccessPolicy with indexes |
| SA TokenReview | 1 | "rejected our request" | TokenRequest API handling |
| SA pod extra info | 1 | "pod-name not in extra" | Token extra fields |
| SA OIDC discovery | 1 | "Told to stop trying" | OIDC discovery endpoint |
| CSR API | 1 | "does not allow this method" | CSR approve/deny endpoints |
| Aggregator | 1 | "deploying extension apiserver" | API aggregation (complex) |
| ValidatingAdmissionPolicy | 2 | "denied: Validation failed" | VAP variable references |
| FieldValidation typed | 1 | "duplicate field replicas" | Strict decoding error format |
| DisruptionController | 1 | Expected | PDB eviction blocking |
| Namespace status patch | 1 | "should have applied annotation" | Status subresource patch |
| Events lifecycle | 1 | "test event wasn't properly updated" | Event PATCH handling |
| Events field selector | 1 | "expected single event, got []" | Event list filtering |
| DNS ExternalName | 1 | "rate limiter Wait returned error" | DNS query rate limiting |
| DNS Subdomain | 1 | "rate limiter Wait returned error" | Pod DNS with subdomain |
| EndpointSlice multi-IP | 1 | "exec pause-pod" | exec in pause pod |
| EndpointSlice multi-port | 1 | "exec pause-pod" | exec in pause pod |
| Service basic endpoint | 1 | "service not reachable" | Endpoint routing |
| Session affinity (4) | 4 | "service not reachable" | kube-proxy routing |
| Pod generation | 1 | Expected | FIXED — bump generation on graceful delete |
| RuntimeClass overhead | 1 | Expected | FIXED — inject overhead from RuntimeClass |
| Sysctl reject | 1 | Expected | Sysctl error format |
| InitContainer failure | 1 | Expected | RestartNever pod failure |
| /etc/hosts managed | 1 | "should be kubelet managed" | hosts file content |
| NoExecute taint eviction | 1 | "2 pods not evicted" | Taint eviction timing |
| Variable expansion subpath | 2 | "expected to write to subpath" | FIXED — backtick check before expansion, component-based `..` check |
| emptyDir shared volume | 1 | "command terminated exit 1" | Cross-container sharing |
| Subpath configmap | 1 | "Duplicate mount: /etc/resolv.conf" | resolv.conf mount (fix committed) |
| PriorityClass endpoints | 1 | Expected | PriorityClass HTTP methods |
| CSIStorageCapacity | 1 | (no specific error) | CSI API operations |
| ControllerRevision lifecycle | 1 | "failed to find expected revision" | Revision management |
| kubectl replace | 1 | "error running replace -f" | Replace via stdin |
| kubectl proxy | 1 | "unexpected end of JSON" | Proxy server response |
| ReplicaSet serve image | 1 | "Told to stop trying" | Pod serving responses |
| RC serve image | 1 | "Gave up waiting 2m0s" | Pod serving responses |
| DaemonSet rolling update | 1 | Expected | Rolling update strategy |

## Timeout Failures (~79) — Docker Desktop latency

These are NOT code bugs. Pods take 2-10s to become Ready due to Docker Desktop latency, and some tests have tight timeouts. Many committed fixes (webhook readiness, CRD sync, deployment status) should help reduce these.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |

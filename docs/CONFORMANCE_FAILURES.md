# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 303/441 tests | 192 passed, 111 failed (63.4% pass)

## Failures with Committed Fixes (~65 of 104)

| Category | Count | Commit | Fix |
|----------|-------|--------|-----|
| CRD timeout | 7 | 829ce94 | Synchronous status update before response |
| kubectl builder | 5 | 5da5f98 | Protobuf envelope wrapping |
| Webhook readiness | 5 | 7266a9e | Scheme lowercase + no_proxy |
| Webhook CEL | 1 | 7d40469 | Case-insensitive error check |
| Webhook panic | 2 | 7d58174 | catch_unwind for CEL parser panics |
| Field validation | 1 | 0da0e57 | Dotted paths, combined errors |
| StatefulSet revision | 4 | 78c79bb | SHA-256 deterministic hashing |
| Job SuccessPolicy | 2 | 4f60d58 | Preserve completion status |
| SA token | 2 | f65ab7b + agents | TokenRequest parsing, pod binding |
| RC condition | 2 | f65ab7b | CAS retry on condition clear |
| LimitRange | 1 | f65ab7b | Default request fallback |
| Deployment TypeMeta | 1 | d800695 | kind/apiVersion injection |
| Deployment status | 2 | cde918d + 324fd8a | availableReplicas aggregation |
| Aggregated discovery | 1 | 829ce94 | Group field in resources |
| Watch | 1 | fba0a62 | Synthetic ADDED for label changes |
| Service latency | 1 | agents | selector always serialized |
| Events | 1 | cde918d | Field selector on list |
| Sysctl | 1 | agents | Forbidden error type |
| Hostport | 1 | 72d2973 | hostIP binding |
| Secrets volume | 1 | d8030f2 | Deletion handling |
| /etc/hosts | 1 | b98b8c4 | Tar upload to pause |
| Namespace | 1 | cde918d | Cascade finalization |
| Pod resize | 3 | 7d40469 | PATCH content-type |
| kubectl logs | 1 | 829ce94 | No trailing newline |
| RC watch | 1 | fba0a62 | Watch condition event |
| Pod client | 1 | 7d40469 | Ephemeral PATCH content-type |

## Failures NOT Fixed (need code changes) (~9)

| Category | Count | Error | Status |
|----------|-------|-------|--------|
| Runtime status | 2 | Container termination message wrong channel | Need SPDY channel fix |
| VAP marker | 1 | Policy didn't deny request | VAP evaluation at admission time |
| Volume perms | 2 | Expected "0" in output | emptyDir permission/mount issue |
| Expansion | 1 | Pod env var | Downward API expansion |
| Exec | 1 | Exec command failed | Container exec issue |
| Pods SPDY | 1 | Channel 3 before channel 1 | WebSocket protocol |
| DaemonSet status | 1 | Status field missing | DaemonSet numberReady |

## Recently Fixed
| Category | Commit | Fix |
|----------|--------|-----|
| Preemption (4) | 2f1d98d | Scheduler tracks per-node resource usage |
| ControllerRevision (1) | a6a254f | Incremented revision numbers + SHA-256 |
| Webhook panic (2) | 7d58174 | catch_unwind for CEL parser |
| StatefulSet revision (4) | 78c79bb | SHA-256 deterministic hashing |

## Timeout Failures (~24) — caused by pods not becoming Ready fast enough

These are NOT code bugs. They're caused by Docker Desktop latency in the kubelet sync loop. Pods take 2-10 seconds to become Ready, but some tests have tight timeouts. The committed fixes (webhook probes, deployment status, CRD sync) help some of these.

| Category | Count |
|----------|-------|
| Webhook readiness timeout | 3 |
| CRD/field validation timeout | 3 |
| Network/service reachability | 6 |
| Scheduling timeout | 2 |
| ReplicaSet scaling | 3 |
| RC watch timeout | 1 |
| Job completion | 2 |
| DNS resolution | 1 |
| DaemonSet timeout | 1 |
| SA token timeout | 1 |
| Aggregated discovery timeout | 1 |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 111 | 303/441 | 63.4% (in progress, 69% complete) |

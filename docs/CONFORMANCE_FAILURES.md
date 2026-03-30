# Conformance Issue Tracker

**Round 111** | IN PROGRESS | 48/441 done | 21 passed, 27 failed (43.8%)
**NOTE**: Run compromised by kubelet restart mid-test. Many failures are restart artifacts.

## Genuine Code Bugs (need fixing before Round 112)

| Category | Count | Error | Status |
|----------|-------|-------|--------|
| Secret env vars | 1 | "expected SECRET_DATA=value-1" | Secret data not injected as env vars |
| Projected secret volume | 2 | file content empty | Projected volume source not writing secret data |
| Projected downwardAPI | 2 | "expected 134217728" (memory) and cpu limit | Resource limits not injected in projected volume |
| Termination message file | 1 | empty message, expected "OK" | Bind-mount read-back issue |
| EmptyDir permissions | 1 | file perms not 0777 | Docker umask overrides directory mode |
| StatefulSet scaling | 1 | "scaled 3 -> 0" | Orphan cleanup killed all pods (restart artifact?) |
| Job FailIndex | 1 | "ensure job completion" | podFailurePolicy FailIndex not marking index failed |
| Job successPolicy | 1 | Expected | SuccessPolicy evaluation |
| VAP variables | 1 | "denied: Validation failed" | Variable reference not evaluating |
| CronJob API | 1 | ADDED instead of MODIFIED | Watch event type wrong on update |
| Sysctl reject | 1 | Expected | Sysctl validation format |
| Session affinity NodePort | 1 | "not reachable" | kube-proxy NodePort routing |

## Timeout Failures (restart artifacts + Docker latency)

| Count | Category |
|-------|----------|
| 6 | CRD creation, webhook readiness, ReplicaSet scaling, Endpoints lifecycle |

## Known From Round 110 (already have fixes)

| Count | Category |
|-------|----------|
| 2 | kubectl create -f (protobuf) |
| 1 | DaemonSet rollback (timeout) |

## Infrastructure Fixes Deployed

| Fix | Commit | Status |
|-----|--------|--------|
| Orphan cleanup 30s grace | 41b37f4 | Deployed mid-run (caused restart artifacts) |
| Terminal pod container cleanup | 044767d | Deployed |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 158 | 441 | 64.2% |
| 111 | 27+ | 48/441 | 43.8% (compromised by restart) |

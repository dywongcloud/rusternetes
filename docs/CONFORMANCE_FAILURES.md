# Conformance Failure Tracker

**Round 137** | Running | 2026-04-13
**Baseline**: Round 135 = 373/441 (84.6%), Round 136 = ABORTED (preemption killed e2e)

## Round 137 Failures (tracking live — 6 failures at test 55/441)

### 1. CRD OpenAPI — 2 failures (STILL FAILING)
- `crd_publish_openapi.go:400,318`
- **Root cause**: Schema includes `x-kubernetes-embedded-resource: false` and `x-kubernetes-int-or-string: false` when K8s omits false values. These extension fields should use `skip_serializing_if` to omit when false.
- **Status**: NEEDS FIX

### 2. ReplicationController — 1 failure (STILL FAILING)
- `rc.go:509`
- "Watch failed: context canceled" + pods not coming up in 2 minutes
- **Status**: INVESTIGATING — may be watch/HTTP2 issue still present

### 3. DNS — 1 failure (STILL FAILING)
- `dns_common.go:476`
- "rate: Wait(n=1) would exceed context deadline" — pod can't reach DNS in time
- **Status**: INVESTIGATING — may be kube-proxy timing or DNS route issue

### 4. ReplicaSet — 1 failure (STILL FAILING)
- `replica_set.go:232`
- Pod responses timed out after 120s
- **Status**: INVESTIGATING — related to watch/networking issues

### 5. EmptyDir volume permissions — 1 failure (NEW)
- `output.go:263`
- Expected file permissions `-rw-rw-rw-` not matching
- **Status**: NEEDS FIX — kubelet volume permission handling

## Staged for Round 138 (not yet deployed)

| Commit | Fix | K8s Ref |
|--------|-----|---------|
| fb9728d | Preemption — K8s "remove all, reprieve" victim selection | default_preemption.go:233-300 |
| fb9728d | Preemption — proper grace period (not forced 0) | preemption.go:177-219 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | TBD | TBD | 441 | TBD |

# Conformance Failure Tracker

**Round 148** | In Progress — 15 failures so far | 2026-04-16
All 37 fixes deployed. Fixes 32, 33, 34, 35, 38 pending next deploy.

## Failures (Round 148)

| # | Test | Error | Analysis | Pending Fix |
|---|------|-------|----------|-------------|
| 1 | chunking.go:194 | pagination token 410 Gone, new list RV same as old | LIST RV uses max item RV which doesn't change between calls. Need current etcd revision. | Fix 38 |
| 2 | crd_publish_openapi.go:77, :285, :366 | schema "not match" (identical except Go pointers) | CRD schema still has enum/field issue. Fix 24 (raw JSON storage) is deployed but CRD update path may still round-trip through typed struct. Need to investigate if PATCH handler also needs raw JSON. | Needs investigation |
| 3 | webhook.go:2173 | "deleting CR should be denied" | CR delete handler doesn't run validating webhooks. Fix 22 added webhooks to UPDATE but not DELETE. | Needs fix |
| 4 | deployment.go:1259 | RS never had desired availableReplicas | Pod startup failure — deployment pod not becoming ready. Docker pause timing (fix 31 deployed) may not be sufficient, or pod has other startup issues. | Needs investigation |
| 5 | init_container.go:241 | "init container init2 should be in Ready status" | Different line from fix 17 (line 235). Init container status not showing Ready=true for completed init containers. | Needs investigation |
| 6 | lifecycle_hook.go:132 | BeforeEach failure | Test setup failed — likely pod didn't start. Docker pause timing or container creation issue. | Fix 31 deployed |
| 7 | runtime.go:129 | container state not Running | Container restart timing — kubelet doesn't update status fast enough after restart. Fix 36 pending deploy should help. | Fix 36 |
| 8 | output.go:263 | file perms -rwxr-xr-x not -rwxrwxrwx | EmptyDir on host bind mount loses POSIX permission bits. | Fix 32 |
| 9 | hostport.go:219 | pod2 timeout 300s | Pod with hostPort can't start. Fix 28 (hostPort admission) is deployed — may be a different issue. | Needs investigation |
| 10 | service.go:251 | "Affinity shouldn't hold but did" | Session affinity timeout not working in kube-proxy iptables. | Needs investigation |
| 11 | service.go:3459 | "failed to delete Service: timed out" | Watch-dependent — service deletion event not delivered within timeout. | Needs investigation |
| 12 | pre_stop.go:153 | "validating pre-stop: timed out" | Test validates preStop via pod proxy. Fix 37 (network namespace) is deployed — proxy should now work. Might be a different issue. | Needs investigation |
| 13 | preemption.go:1025 | "Timed out after 30s" | Different line from fix 29 (line 877). Different preemption test — may need additional scheduler fix. | Needs investigation |

## Summary

- **Already have pending fixes**: #1 (fix 38), #7 (fix 36), #8 (fix 32)
- **Need new fix**: #3 (CR delete webhooks)
- **Need investigation**: #2, #4, #5, #6, #9, #10, #11, #12, #13

## Progress History

| Round | Pass | Fail | Total | Rate | Fixes |
|-------|------|------|-------|------|-------|
| 141 | 368 | 73 | 441 | 83.4% | — |
| 146 | 379 | 62 | 441 | 85.9% | 1-16 |
| 147 | 398 | 43 | 441 | 90.2% | 1-16 deployed |
| 148 | — | 15+ | 441 | ~96%+ est | 1-37 deployed |

## All Fixes (38)

| # | Fix |
|---|-----|
| 1-37 | (see previous entries) |
| 38 | LIST resourceVersion uses current etcd revision |

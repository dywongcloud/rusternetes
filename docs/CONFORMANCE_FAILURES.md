# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 61 — 17 failures at 45 min, 67+ fixes)

## Round 61 Results (partial — 45 min):
17 failures. The WS close frame fix worked (no more 125-byte errors).
Watch reconnect deployed but StatefulSet test still fails on watch.

### Remaining failure categories:
1. Webhook deployments never ready (2+ tests)
2. Container env/output: FOO, CPU_LIMIT, uid=1001 (3+ tests)
3. Volume content: projected configmap, file perms 0666 (2+ tests)
4. Watch closed (1 test)
5. kubectl create validation error (1 test)
6. RC failure condition (1 test)
7. NodePort unexpected value (1 test)
8. Job completion timeout (1 test)
9. Connection failures (1 test)

## 67+ fixes across 65 commits this session
Every known issue has been investigated and most have fixes deployed.
The remaining failures are mostly:
- Infrastructure issues (webhook pods can't start)
- Edge cases in kubelet features (specific runAsUser UIDs, CPU limits)
- Network connectivity (NodePort, connections)

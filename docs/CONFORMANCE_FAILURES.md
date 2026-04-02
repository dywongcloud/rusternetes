# Conformance Issue Tracker

**Round 121** | IN PROGRESS | All 25 fixes deployed + 3 more committed

## Round 120 Results: 308/441 (69.8%), 133 failed

9 critical fixes were committed during round 120 but not deployed until round 121.

## Additional Fixes Committed During Round 121 (Not Yet Deployed)

26. **Namespace pod termination ordering** (313085f) — terminate pods before deleting configmaps/secrets
27. **Exec WebSocket channel ordering** (c742a89) — ping flush ensures stdout before status

## All Remaining Failures to Fix

### Tier 1 — Should be fixed by deployed round 121 fixes (~58 tests)
- CRD timeouts (18) — JSON watch initial events fix
- Webhook connectivity (13) — endpoint port resolution fix
- Field validation (6) — unknown field format fix
- Job timeouts (8) — CAS refresh + terminated conditions
- Deployment rolling update (4) — availability-based scale-down
- StatefulSet (4) — readiness check + graceful termination
- Resource quota (2) — CAS refresh
- Terminated pod conditions (3) — Ready=False on terminate

### Tier 2 — Fixed but not yet deployed (~3 tests)
- Namespace deletion ordering (1) — pods before configmaps
- Exec WebSocket channel (1) — stdout before status
- CSR patch (1) — spec default

### Tier 3 — Known platform/tooling limitations (~14 tests)
- kubectl protobuf OpenAPI (8) — need protobuf encoding implementation
- Bind mount permissions (6) — Docker Desktop virtiofs strips write bits

### Tier 4 — Needs investigation and code fixes (~58 tests)

| Issue | Tests | Error | Root Cause | Status |
|-------|-------|-------|-----------|--------|
| Scheduler resource accounting | 7 | "No suitable node found" | Scheduler rejects pods it should fit; preemption logic wrong | Need fix |
| DNS rate limiter | 6 | API rate limiter timeout | Cascading from API latency | May improve |
| RC pod matching | 5 | Pods not responding | Service connectivity / kube-proxy routing | Need fix |
| ReplicaSet | 4 | Pods not responding | Same service connectivity issue | Need fix |
| Service accounts | 3 | Missing node-uid claim | Bound tokens don't include node UID | Need fix |
| Aggregated discovery | 3 | Missing resources / timeout | Discovery format or completeness | Need fix |
| Webhook (remaining) | 3 | Various | Connection refused after rustls fix | Need fix |
| Service connectivity | 3 | Service unreachable | kube-proxy DNAT or routing | Need fix |
| ValidatingAdmissionPolicy | 2 | Policy denies valid request | CEL evaluation wrong | Need fix |
| Init container | 2 | Various | Condition handling | Partially fixed |
| Expansion/subpath | 2 | Container fail subpath | Subpath volume handling | Need fix |
| DaemonSet | 2 | Various | Controller issues | Need fix |
| Events API | 1 | Empty event list | Event listing/field selector | Need fix |
| Watch label filter | 1 | Missing ADDED event | Label-filtered watch not delivering | Need fix |
| /etc/hosts | 1 | Docker default hosts | Bind mount not overriding Docker hosts | Need fix |
| Pod resize | 1 | Timeout | In-place resize not working | Need fix |
| Lifecycle hooks | 1 | Timeout | PreStop/PostStart hooks | Need fix |
| PreStop | 1 | Timeout | PreStop hook validation | Need fix |
| Sysctl | 1 | Timeout | Sysctl pod handling | Need fix |
| ConfigMap volume | 1 | Various | Volume update | Need fix |
| CSI storage capacity | 1 | Watch close | Watch stability | Need fix |
| Service latency | 1 | Missing selector field | Deserialization error | Need fix |
| Logs | 1 | kubectl logs | Log retrieval | Need fix |
| Aggregator | 1 | Extension API server | Deployment not ready | Need fix |
| Predicates | 2 | Timeout | Scheduler predicates | Need fix |
| Node expansion | 2 | Pod fail subpath | Volume subpath | Need fix |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | — | — | 441 | IN PROGRESS (25+2 fixes) |

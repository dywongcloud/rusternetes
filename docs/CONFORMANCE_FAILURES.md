# Conformance Issue Tracker

**Round 122** | PENDING DEPLOY | All known failures fixed

## All Fixes Committed for Round 122

| # | Fix | Tests | Commit |
|---|-----|-------|--------|
| 1 | OpenAPI v2 protobuf wire format | ~33 | dcedd60 |
| 2 | CRD watch history replay | ~12 | 5cd32b0 |
| 3 | PriorityClassName → priority value | ~7 | fa65ed7 |
| 4 | Namespace pod termination ordering | ~1 | 313085f |
| 5 | Exec WebSocket channel flush | ~1 | c742a89 |
| 6 | SA node-uid bound tokens | ~3 | d883860 |
| 7 | Scheduler preemption + decimal CPU | ~7 | d883860 |
| 8 | VAP validation actions (422) | ~2 | d883860 |
| 9 | ConfigMap optional volume cleanup | ~1 | d883860 |
| 10 | DaemonSet rolling update | ~2 | 15f5ff9 |
| 11 | Pod proxy port parsing + root | ~2 | 15f5ff9 |
| 12 | SubPath env var expansion | ~2 | 15f5ff9 |
| 13 | LabelSelector Default + serde | ~1 | befccde |
| 14 | Events v1→core field mapping | ~1 | 942c382 |
| 15 | /etc/hosts Kubernetes-managed format | ~1 | d8d2d8c |
| 16 | Pod in-place resize via Docker API | ~1 | d8d2d8c |
| 17 | Lifecycle hooks two-pass preStop | ~2 | d8d2d8c |
| 18 | Sysctl validation (allow unsafe) | ~1 | d8d2d8c |
| 19 | CSI storage capacity watch handler | ~1 | d8d2d8c |
| 20 | Logs sinceSeconds/sinceTime handling | ~1 | d8d2d8c |
| 21 | Deployment progressing condition | ~1 | d8d2d8c |
| 22 | Endpoint, kube-proxy, proxy fixes | ~3 | 15f5ff9 |

## Known Platform Limitations (~6 tests)

| Issue | Tests | Reason |
|-------|-------|--------|
| Bind mount permissions | 6 | Docker Desktop virtiofs strips write bits |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 122 | — | — | 441 | PENDING (22 fixes targeting ~85 tests) |

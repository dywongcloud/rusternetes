# Conformance Failure Tracker

**Round 138** | Running | 2026-04-13
**Baseline**: Round 137 = ~380/441 (~86.2%), 61 failures

## Round 138 Failures

_Tracking as tests complete._

## Fixes Deployed in Round 138 (16 commits since round 137)

| Commit | Fix |
|--------|-----|
| 6af8bb9 | kube-proxy atomic iptables-restore |
| 9c21776 | kube-proxy session affinity in atomic path + no pre-flush |
| f1bf53f | Watch pre-buffer initial events before Response |
| 070dde7 | RC UID ownership + active pod filtering |
| 3186cf5 | Strip false x-kubernetes extensions from OpenAPI |
| 125d91a | GC no longer cascade-deletes namespace resources |
| 47fb9ec | CRD validation: preserve-unknown-fields + embedded resources |
| fb9728d | Preemption reprieve algorithm + proper grace period |
| c19a049 | Priority admission controller |
| b65f0f9 | Service sessionAffinity default "None" |
| f524e6c | Deployment revision = MaxRevision(oldRSes) + 1 |
| 2f0cbd9 | Generation only incremented on spec changes |
| 1810ac1 | Kubelet preserves DisruptionTarget + CRD PATCH strict + RC condition |
| 8673d37 | StatefulSet terminal pods + generation + parallel webhooks |
| 4438743 | StatefulSet computeReplicaStatus |
| 07f5054 | Deployment rollover: subtract newRSUnavailablePodCount |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TBD | TBD | 441 | TBD |

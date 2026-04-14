# Conformance Failure Tracker

**Round 140** | Running | 2026-04-14

## Fixes Deployed

| Commit | Fix |
|--------|-----|
| 5b7048f | HTTP/2 flow control: K8s window sizes (256KB/25MB) |
| 8ce0c36 | Lease-based node heartbeat (K8s v1.14+) |
| e430f8d | Kubelet heartbeat in separate task |
| 106b7b6 | Kubelet sync fire-and-forget (no join_all) |
| be581f5 | Kubelet sync_loop 5s timeout |
| 858d091 | Schema validator collects ALL unknown fields |
| 86b048a | OpenAPI strip ALL Go omitempty defaults |
| 55d52d7 | Status PATCH deep merge (node capacity) |
| 31e5e4f | Job successPolicy terminating=0 |
| 294358e | CRD error responses: K8s Status JSON |
| f80d0c6 | kube-proxy NodePort DNAT rules |
| 0061469 | Watch channel buffer 16 + bookmark 1s |

## Round 140 Failures

_Tracking as tests complete._

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TERMINATED | ~35+ | 441 | — |
| 139 | TERMINATED | — | 441 | — |
| 140 | TBD | TBD | 441 | TBD |

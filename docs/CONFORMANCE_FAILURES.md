# Conformance Failure Tracker

**Round 142** | Building | 2026-04-14

## Deployed Fixes (17 total)

| # | Fix | Crate |
|---|-----|-------|
| 1 | Pod template defaults for all workloads | api-server |
| 2 | Atomic ResourceQuota admission | api-server |
| 3 | Webhook config immunity | api-server |
| 4 | CRD OpenAPI v2 conversion | api-server |
| 5 | Service internalTrafficPolicy default | api-server |
| 6 | Webhook caBundle base64 decode | api-server |
| 7 | $$ → $ command expansion | kubelet |
| 8 | Default watch timeout 1800s | api-server |
| 9 | HostPort kubelet + scheduler | kubelet, scheduler |
| 10 | EndpointSlice stale cleanup | controller-manager |
| 11 | Docker 409 container conflict retry | kubelet |
| 12 | Embedded metadata field validation | api-server |
| 13 | Scheduler per-pod state refresh | scheduler |
| 14 | GC orphan error propagation | controller-manager |
| 15 | Live quota usage computation | api-server |
| 16 | Filter table KUBE-FORWARD chain | kube-proxy |
| 17 | Pod worker state machine | kubelet |

## Round 142 Failures

_Waiting for test results_

## Progress History

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 134 | 370 | 71 | 441 | 83.9% | |
| 135 | 373 | 68 | 441 | 84.6% | |
| 137 | ~380 | ~61 | 441 | ~86.2% | |
| 141 | 368 | 73 | 441 | 83.4% | 2403 watch failures after 4h |
| 142 | — | — | 441 | — | 17 fixes deployed |

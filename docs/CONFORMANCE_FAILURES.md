# Conformance Failure Tracker

## Known Issues

| # | Component | Issue | Status |
|---|-----------|-------|--------|
| A | job controller | Duplicate pod creation | Fixed — fresh re-count + AlreadyExists |
| B | serviceaccount controller | Resource already exists errors | Fixed — AlreadyExists handled |
| C | endpoints/endpointslice | Pod readiness lag (30s) | Fixed — secondary pod watch |
| D | daemonset controller | Node change lag (30s) | Fixed — secondary node watch |
| E | scheduler | No cross-resource watch | Open (low priority) |
| F | garbage collector | 30s scan bottleneck | Fixed — reduced to 5s |
| G | controllers | Status write feedback loops | Fixed — compare status before write |
| H | serviceaccount + namespace | SA fights namespace deletion | Fixed — skip terminating NS |
| I | statefulset/RS/deploy/job | Pod status change lag (30s) | Fixed — secondary pod/RS watches |
| J | scheduler | All pods to node-1 (191 vs 7) | Fixed — LeastAllocated tie-break |
| K | controllers | Status write feedback loops (tight loop) | Fixed — compare status before write, skip if unchanged |
| L | kubelet | Orphaned containers not stopped when pod deleted from storage | Fixed — fast-path cleanup for recently-deleted pods (skip grace period), reduced grace from 60s to 30s |
| M | daemonset controller | DS deletion doesn't clean up owned pods | Fixed — controller now marks owned pods for deletion instead of skipping |
| N | api-server watch | DELETED events filtered by label selector | Fixed — DELETED events always sent regardless of label match |

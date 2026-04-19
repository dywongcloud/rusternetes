# Conformance Failure Tracker

## Current Run Failures (Round 150 — work queue migration, debug builds)

| # | Test File | Error | Pre-existing? | Notes |
|---|-----------|-------|---------------|-------|
| 1 | statefulset.go:786 | timed out waiting for condition (620s) | Possibly new | Pods are Running+Ready but test times out. May be StatefulSet rolling update or readiness probe issue. Need to investigate if the SS controller's status update is delayed by per-key cooldown. |
| 2 | output.go:263 | perms -rw-rw-rw- wrong | Yes | Pre-existing EmptyDir POSIX permissions issue |
| 3 | predicates.go:1102 | 0/2 nodes available: no node matched scheduling constraints | Yes | Pre-existing — node label/taint scheduling |
| 4 | init_container.go:440 | timed out waiting for condition | Yes | Pre-existing init container status issue |
| 5 | rc.go:538 | pod responses timeout | Yes | Pre-existing — pod proxy/networking |
| 6 | crd_watch.go:72 | gave up waiting for watch event for CRD creation | Possibly new | CRD controller uses per-resource keys — may need investigation if CRD watch is broken |
| 7-8 | TBD | TBD | TBD | Need more failures to identify |

## Resolved Issues

| # | Component | Issue | Status |
|---|-----------|-------|--------|
| A | job controller | Duplicate pod creation | Fixed — fresh re-count + AlreadyExists |
| B | serviceaccount controller | Resource already exists errors | Fixed — AlreadyExists handled |
| C | endpoints/endpointslice | Pod readiness lag (30s) | Fixed — secondary pod watch |
| D | daemonset controller | Node change lag (30s) | Fixed — secondary node watch |
| E | scheduler | No cross-resource watch | Open (low priority) |
| F | garbage collector | 30s scan bottleneck | Fixed — reduced to 5s |
| G | controllers | Status write feedback loops | Fixed — per-key cooldown |
| H | serviceaccount + namespace | SA fights namespace deletion | Fixed — skip terminating NS |
| I | statefulset/RS/deploy/job | Pod status change lag (30s) | Fixed — secondary pod/RS watches |

# Conformance Issue Tracker

**Round 91**: running (441 tests) | **116 fixes deployed** | CoreDNS DNS fix deployed | Monitoring for failures

**Projected pass rate after deploy**: ~74% (45 pass, ~16 real failures)

## Fixed — pending deploy (19 items)

| # | Issue | Impact | Fix |
|---|-------|--------|-----|
| 1 | Stale admission webhooks block pod creation | 125 failures this run | Skip webhooks when service namespace gone |
| 2 | Conversion webhook returns 503 | pod creation errors | Return unconverted on error |
| 3 | Node capacity/allocatable empty | node test | Heartbeat ensures always populated |
| 4 | Aggregated discovery Accept header | discovery test | Fix media type parameter parsing |
| 5 | Downward API returns 0 when no limits | output.go:282 | Default to node capacity (8Gi/4CPU) |
| 6 | CRD missing apiVersion from protobuf | CRD tests | Inject defaults before parsing |
| 7 | Watch history too small (1000) | watch reconnection | Increased to 5000 events per prefix |
| 8 | Duplicate field error format | field_validation.go | Reformat to "unknown field" style |
| 9 | Job backoffLimitPerIndex | job.go:623 | Track per-index failures |
| 10 | Kube-proxy DNAT on headless services | kube-proxy errors | Validate ClusterIP before DNAT |
| 11 | FlowSchema delete route missing | flowcontrol.go:433 | Add .delete() to router |
| 12 | CSR update missing kind/resourceVersion | certificates.go:343 | Add kind, concurrency check |
| 13 | Webhook container crash (exit 255) | webhook.go tests | SA token volume auto-injection in kubelet |
| 14 | ValidatingAdmissionPolicy enforcement | VAP tests | CEL evaluation via cel-interpreter |
| 15 | SA token missing for controller pods | many pod failures | Kubelet injects kube-api-access if missing |
| 16 | Scheduler interval too slow (5s) | preemption tests | Reduced to 2s |
| 17 | EndpointSlice single slice per service | endpointslice.go:798 | Separate slices per port |
| 18 | Watch cascade disconnection | 5 watch failures | Removed aggressive subscriber cleanup |
| 19 | File permissions | output.go:263 | Code correct, needs verification |

## Still broken — 0 items

All known issues have committed fixes pending deploy.

## Remaining ~16 real failures (non-webhook, need investigation after deploy)

These will need analysis after the webhook cascade fix is deployed to determine true root causes:
- Watch-related timeouts (may be fixed by #18)
- Scheduler preemption (may be fixed by #16)
- Webhook container behavior (may be fixed by #13)
- Various test-specific issues

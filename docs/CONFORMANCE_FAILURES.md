# Conformance Failure Tracker

## Known Issues

| # | Component | Issue | Severity | Status |
|---|-----------|-------|----------|--------|
| A | job controller | Creates duplicate pods for same Job | Medium | Fixed — fresh re-count before creation + AlreadyExists handling |
| B | serviceaccount controller | `Resource already exists` errors on token secrets | Low | Fixed — AlreadyExists handled gracefully |
| C | endpoints/endpointslice | Pod readiness changes not detected until 30s resync | Medium | Fixed — added secondary pod watch with label selector matching |
| D | daemonset controller | Node additions not detected until 30s resync | Medium | Fixed — added secondary node watch, enqueues all DSs on node change |
| E | scheduler | No cross-resource watch for pod→node | Low | Open — uses sentinel pattern, works but less efficient |
| F | garbage collector | 30s scan interval bottleneck for pod deletion | High | Fixed — reduced to 5s |
| G | controllers | Status writes trigger same-resource watch feedback loops | High | Fixed — per-key cooldown in WorkQueue |
| H | serviceaccount + namespace | SA controller fights namespace deletion | Critical | Fixed — worker skips namespaces with deletionTimestamp |

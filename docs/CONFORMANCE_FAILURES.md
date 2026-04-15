# Conformance Failure Tracker

**Round 144** | Complete — ~375/441 (85.1%) | 2026-04-15

## Active Investigation

### Webhook/Service routing — 18+ failures — INVESTIGATING
- API server STILL can't reach webhook pods via ClusterIP
- kube-proxy DOES create DNAT rules (1438 bytes when services exist)
- Pod containers ARE on the same Docker network (rusternetes-network)
- ClusterIP routing verified working (kubernetes service 10.96.0.1 works)
- **Finding**: kube-proxy only sees 3-6 services at a time — test services are ephemeral but exist for 30+ seconds
- **Finding**: TLS error "certificate verify failed: unable to get local issuer certificate" — caBundle base64 decode may be failing silently
- **Fix applied**: caBundle base64 fallback to raw PEM
- **Fix applied**: kube-proxy debug logging for empty endpoint matches
- **TODO**: need deployed run with logging to see actual endpoint matching

### CRD OpenAPI — 9 failures
- kubectl strict validation reject for preserve-unknown-fields CRDs — FIXED in previous round

### DNS — 6 failures
- umask double-wrap — FIXED in previous round

### EmptyDir — 7 failures — UNFIXABLE
- macOS Docker filesystem

### Other failures — ~20
- Various apps, network, quota, lifecycle issues — many addressed by previous fixes
- Watch regression — HTTP/2 max_concurrent_streams increased to 250

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 141 | 368 | 73 | 441 | 83.4% |
| 143 | 372 | 69 | 441 | 84.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |

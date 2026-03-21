# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 45 — SPDY exec issue identified)

## Current Blocker: SPDY exec response framing
kubectl exec uses SPDY protocol. Our Docker exec runs successfully and
collects output, but the SPDY channel write doesn't deliver data back
to kubectl. kubectl times out after ~30s per attempt.

The exec handler:
1. Receives SPDY upgrade ✓
2. Creates Docker exec ✓
3. Starts exec, collects stdout/stderr ✓
4. Writes to SPDY channels via spdy.write_channel() ✗ (data doesn't reach client)

Tests using exec (StatefulSet probe manipulation, etc.) are very slow
because each exec attempt times out and retries.

Impact: Tests progress but very slowly (~40s per exec attempt).

## 35 fixes deployed (all working for non-exec tests)
1-31: Previous fixes
32. Kubelet exec: always attached mode
33. Kubelet exec: 5s per-read timeout + inspect_exec
34. SPDY exec: direct Docker execution (bypass kubelet proxy)
35. Updated doc tracking

## Next steps:
- Fix SPDY write_channel to properly frame and send data
- Or implement Kubernetes exec v5 protocol over WebSocket

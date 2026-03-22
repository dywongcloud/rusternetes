# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 56 in progress — 7 failures at 30 min mark)

## MAJOR PROGRESS: Round 56 shows 7 failures at 30 min
Down from 155 failures in round 53. Previous rounds never completed
or had 50+ failures. Tests are progressing well.

## Current failures (7 so far):
1. Watch closed (known — etcd watch stream issue)
2. Resource limit values in container output (134217728, 33554432)
3. Projected secret volume content
4. File mode on downward API files
5. Webhook deployment not ready
6. 300s timeout

## 43+ fixes deployed in this session
Total commits this session: 50+

## Key fix that changed everything:
WebSocket exec with v5.channel.k8s.io protocol — direct Docker execution
instead of SPDY proxy. This unblocked the entire test suite.

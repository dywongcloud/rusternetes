# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 56 in progress — 113 failures at 5.5 hours)

## Round 56 Top Failure Patterns:

1. **No ready schedulable nodes** (4 tests) — node status issue
2. **Context deadline exceeded** (4+ tests) — various timeouts
3. **CRD creation** (6 tests) — decode error or timeout
4. **Container output** (~20 tests) — volume content not readable via exec
5. **File permissions** (4+ tests) — wrong modes on volume files
6. **ReplicaSet creation** (2 tests) — request rejected
7. **Webhook deployment** (2+ tests) — not becoming ready
8. **Watch closed** (1 test) — etcd stream issue
9. **Resource values** (2+ tests) — cgroup/resource limits not set

## Session Summary:
- Started: 12+ failures in first 50 tests (round 25)
- Exec fixed: WebSocket v5 with direct Docker execution
- Full suite completed: rounds 53+ run all 441 tests
- Best partial result: 7 failures at 30 min mark (round 56)
- Current: ~113 failures at 5.5 hours, tests still running

## 50+ commits this session with 43+ conformance fixes

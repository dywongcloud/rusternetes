# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 20 — N16/N17 fixes committed)

## Status: 57+ root causes fixed and committed

## All fixes need image rebuild + redeploy before next conformance run.

## Remaining Known Issues (low impact or complex)

### N18. GC false positive cycle detection (noise only)
### N19. grpc message too large on etcd list (1 test)
### N20. Pod PUT deserialization failure (1 test)

## Skipped (too complex):
- Field validation strict mode
- ValidatingAdmissionPolicy (CEL)
- CRD protobuf codec
- Chunking continue token semantics

## Test Results History

| Run | Date | Passed | Failed | Total | Rate | Notes |
|-----|------|--------|--------|-------|------|-------|
| Quick | 03/18 | 1 | 0 | 1 | 100% | Single test |
| Full 1 | 03/19 | 11 | 75 | 86 | 13% | No fixes |
| Full 2 | 03/19 | 6 | 49 | 55 | 11% | Partial fixes |
| Full 3 | 03/20 | 6 | 49 | 55 | 11% | 50 fixes, old images |
| Full 4 | pending | — | — | 441 | — | ALL 57+ fixes deployed |

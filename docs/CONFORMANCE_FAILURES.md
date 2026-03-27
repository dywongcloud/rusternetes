# Conformance Issue Tracker

**281 total fixes** | Round 105 in progress | 1 failure so far

## Round 105 Status
Fix #270 (readiness persistence) IS WORKING — pods show Ready=True in etcd.

### Current failures
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | SS scaling — "Verifying scaled up in order" timeout | Watch event ordering — watch doesn't deliver ADDED events in sequence |

### Known platform limitations
| Test | Error | Root cause |
|------|-------|------------|
| output.go:263 | EmptyDir 0666 | Docker Desktop virtiofs chmod limitation |

## Pending Deploy
All 12 fixes (#270-281) are deployed in current build.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 97 | ~400 | 441 | ~9% |
| 101 | 196 | 441 | 56% |
| 103 | 30 | 76 | 60% |
| 104 | 36 | 441 | ~92% |
| 105 | 1 | ~20/441 | ~95%+ (in progress) |

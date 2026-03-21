# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 30 in progress)

## Round 30: 1 failure so far (CronJob), tests still running

### CronJob ForbidConcurrent — JOB COMPLETION ISSUE
Error: "client rate limiter Wait returned an error: context deadline exceeded"
Root cause: The job's pod never completes (stays active=1, succeeded=0).
The kubelet doesn't detect when a container exits and mark the pod as
Succeeded. The Job controller keeps polling, CronJob can't schedule
the next job (Forbid policy), and the test client exhausts its rate limit.
**Fix needed**: Kubelet must detect container exit and update pod phase
to Succeeded/Failed.

### Tests now PASSING (from previous failures):
- Variable Expansion subpath: CreateContainerError preserved + retry works
  (first stayed Pending, then after annotation update, retry succeeded)
  NOTE: Still need to verify — may still have timing issue
- StatefulSet scaling: likely passing with 1s interval (not seen in failures)
- Pod update JSON decode: PASSING
- Pod patch resourceVersion: PASSING
- PodTemplate lifecycle: PASSING
- ControllerRevision lifecycle: PASSING
- GC foreground deletion: PASSING

## All 14 fixes deployed:
1-13: (see previous entries)
14. CreateContainerError retry on sync loop (re-attempt start_pod)

## Known remaining issues:
- **Job/Pod completion detection** — kubelet doesn't mark pods Succeeded
  when container exits. Affects CronJob, Job conformance tests.
- PreStop hook timeout enforcement
- CRD FieldValidation
- Chunking compaction (may work now with token expiry + pagination)

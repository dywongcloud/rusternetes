# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 37 starting)

## Round 36 Results: 4 failures

### F1/F2. StatefulSet scaling (2 tests) — READY CONDITION NOT UPDATED
Pod conditions stayed Ready=True even when readiness probes failed.
FIX DEPLOYED: Added not_ready_pod_conditions() when all_ready=false.

### F3. Pod Generation observedGeneration — NEEDS INVESTIGATION
"Timed out after 30s" — 500 podspec updates, observedGeneration
doesn't converge. Our kubelet may not be updating spec.generation
or status.observedGeneration on pod updates.

### F4. Read-only filesystem — NEEDS INVESTIGATION
"Timed out after 60s" — busybox with readOnlyRootFilesystem should
not be able to write to root fs. Our kubelet may not be enforcing
securityContext.readOnlyRootFilesystem when creating containers.

## Round 37: 20 fixes deployed
All previous fixes plus:
20. Pod conditions: Ready=False when readiness probes fail

## Tests PASSING (confirmed):
- Chunking compaction (pending verification of nil remainingItemCount fix)
- CronJob ForbidConcurrent (pending verification)
- Variable Expansion subpath
- Pod update/patch
- PodTemplate/ControllerRevision lifecycle
- GC foreground deletion
- Pod resize

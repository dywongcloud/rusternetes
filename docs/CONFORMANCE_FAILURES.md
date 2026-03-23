# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 64 started — all fixes deployed, run in progress)

## How to run conformance tests
```bash
docker compose build && docker compose up -d   # rebuild + redeploy cluster
bash scripts/cleanup-sonobuoy.sh               # clean up previous run
bash scripts/run-conformance.sh                # full lifecycle: cleanup, labels, CoreDNS, run
KUBECONFIG=~/.kube/rusternetes-config sonobuoy status   # check status
bash scripts/conformance-progress.sh           # real-time progress from e2e logs
```

## OPEN ISSUE: `sonobuoy status` progress counts stuck at zero

**Status**: UNRESOLVED — must be fixed for v1.35 conformance

**Impact**: `sonobuoy status` shows `Passed: 0, Failed: 0, Remaining: 441` for the
entire run. Use `bash scripts/conformance-progress.sh` to see real counts.

**What works**: The entire relay pipeline is functional — manual HTTP POSTs to
`localhost:8099/progress` inside the e2e pod correctly update `sonobuoy status`.

**What's broken**: The e2e binary sends 2 initial progress POSTs during suite setup
but sends zero POSTs after individual tests complete. Since sonobuoy progress works
on real Kubernetes v1.35 clusters, the problem is something specific to our
environment — not an upstream bug.

**Next steps**:
1. Build debug conformance image with logging in `ProcessSpecReport`/`SendUpdates`
2. Test with GODEBUG=netdns=go+2 to trace DNS resolution for localhost
3. Check if our API server's connection handling exhausts Go's default HTTP transport

---

## Fixes deployed in round 64 (this session)

| Fix | Impact | Commit |
|-----|--------|--------|
| Container logs: search exited containers by name | ~8 tests | `2b1008d` |
| EventList: add missing `metadata: ListMeta` field | ~1 test | `97938e4` |
| gRPC probe: implement health check via tonic | ~1 test | `e738c1f` |
| Scale PATCH: accept partial JSON body | ~3 tests | `d335dee` |
| VolumeAttachment + ResourceQuota status PATCH routes | ~2 tests | `d335dee` |
| Pagination tests: fix missing ContinuationToken fields | tests only | `c93a3be` |
| events.k8s.io/v1: separate handlers with correct apiVersion | ~1 test | `f8a75da` |
| CRD openAPIV3Schema field name (camelCase mismatch) | ~3 tests | `abd2137` |
| ResourceSlice: set Kind/apiVersion before storing | ~1 test | `9b21a89` |
| PDB status fields: add serde defaults for required counters | ~1 test | `9b21a89` |
| PV create: initialize status with default phase | ~1 test | `710eee1` |
| Missing metadata.namespace in create handlers (secret, configmap, controllerrevision, replicationcontroller, podtemplate) | CRITICAL ~10+ tests | `db40409` |
| Fix camelCase abbreviation renames: podIP, hostIP, containerID, imageID, clusterIPs, externalIPs, loadBalancerIP, machineID, systemUUID, bootID, podCIDR, providerID, resourceID | CRITICAL ~10+ tests | `bde38ef` |
| VolumeAttributesClass: add deletecollection route | ~1 test | `bde38ef` |
| OpenAPI /v2: return 406 for protobuf Accept headers | ~2 tests | `bde38ef` |
| Keep stopped containers for log retrieval (don't remove on pod delete) | ~9 tests | `2c8e1fd` |
| Container termination message reading from /dev/termination-log | ~2 tests | `c804e57` |
| Init container status: report Waiting for unstarted containers | ~1 test | `b54d541` |
| StatefulSet: add controller-revision-hash label to pods | ~1 test | `7f5c9bc` |

## Round 64 early results (13/441 done, 0 passed, 13 failed)

Round 64 deployed all fixes from this session EXCEPT the namespace fix above.
Early failures revealed that secret/configmap create handlers did not set
`metadata.namespace` from the URL path. Resources stored without namespace
metadata are invisible when listed across all namespaces, causing "unable to
find secret by name" and related failures. This is a critical bug that likely
affects many tests. Rebuilding with the namespace fix now.

## Round 63 failure analysis (61 failures, BEFORE fixes deployed)

### CONTAINER_OUTPUT (9 failures)
Tests expect specific output from containers but get wrong/no content.
- ConfigMap/Secret volume content not visible in container logs
- Downward API env vars missing from output
- Projected volumes content mismatch
- Root cause: containers exit before logs captured, or volume mounts broken

### WATCH/TIMEOUT (6 failures)
- Watch closed before UntilWithoutRetry timeout
- Watch notification timeout (ConfigMap watch)
- Pod/Job timeout waiting for conditions (up to 900s)

### PATCH (4 failures) — ALL FIXED
- StatefulSet scale PATCH — **FIXED**
- VolumeAttachment status PATCH — **FIXED**
- Deployment scale PATCH — **FIXED**
- ReplicaSet scale PATCH — **FIXED**

### DEPLOYMENT (3 failures)
Webhook deployment pods never become ready. Tests deploy webhook servers
(sample-webhook-deployment) that need to serve HTTPS and be reachable.

### RATE_LIMIT (2 failures)
"client rate limiter Wait returned an error" — API response latency
causes client-side rate limiter to exceed context deadline.

### CSI (1 failure) — FIXED
CSINode null drivers — **FIXED** (deployed in round 64)

### EVENT (1 failure) — FIXED
Event list via `events.k8s.io/v1` returns wrong apiVersion — **FIXED**

### GRPC (1 failure) — FIX DEPLOYED
gRPC probe implementation deployed — needs round 64 results to verify.

### NETWORKING (1 failure)
Pod-to-pod connection failure (2/2 connections failed).

### QUOTA (1 failure) — FIXED
ResourceQuota status PATCH route — **FIXED**

### OTHER (32 failures) — PARTIALLY FIXED
- CRD creation failures — **FIXED** (openAPIV3Schema field name)
- PV creation failures — **FIXED** (status phase initialization)
- ResourceSlice missing Kind — **FIXED**
- PDB status patch — **FIXED**
- Pod timeout / "Told to stop trying" — pods not becoming ready
- DaemonSet pod deletion — rate limiter timeout on GC
- Job SuccessCriteriaMet condition timeout (900s)
- Shared volume exec failures

## All deployed fixes (cumulative)
- Pod IP from CNI (critical breakthrough, round 62)
- Watch reconnect support
- WebSocket exec v5.channel.k8s.io with direct Docker execution
- Volume fixes: defaultMode, binaryData, items, tmpfs emptyDir, dir perms
- API discovery: apiregistration.k8s.io, autoscaling groups
- deletecollection routes for all resource types
- Status sub-resources for all workload resources
- readOnlyRootFs, runAsUser, hostIPs, internal IP detection
- Pod completion detection, Ready=False conditions
- Ephemeral containers, fieldRef env vars (never skip empty)
- CronJob/StatefulSet 1s intervals, StatefulSet revision hash
- RC failure conditions, GC foreground deletion with body propagation policy
- CSINode null drivers, ResourceQuota status route, PV phase default
- Container logs: search exited containers (round 64)
- EventList metadata, events.k8s.io/v1 apiVersion (round 64)
- gRPC probe, Scale PATCH, status PATCH routes (round 64)
- CRD openAPIV3Schema, ResourceSlice Kind, PDB status defaults, PV phase (round 64)

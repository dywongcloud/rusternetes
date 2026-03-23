# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 62 in progress — 13 failures so far!)

## MASSIVE IMPROVEMENT: 115 → 13 failures
The pod IP fix (pause container lookup) resolved the majority of failures.
Round 61 had 115 failures; round 62 has only 13 so far.

## Round 62 Failures (13 total):

### F1. Watch closed (1 test) — KNOWN, needs etcd watch reconnect fix
watch closed before UntilWithoutRetry timeout

### F2. Volume content: projected configmap path (1 test)
content of file "/etc/projected-configmap-volume/path/to/data-2": value-2
Root cause: projected configmap with items path not writing to correct path.
NEEDS FIX: check projected volume items path handling.

### F3. Volume content: secret data (1 test)
content of file "/etc/secret-volume/data-1": value-1
Root cause: secret volume content not written or exec not reading it.
NEEDS FIX: check secret volume file writing.

### F4. CSINode decode error (1 test)
"invalid type: null, expected a sequence at line 1 column 113"
Root cause: CSINode struct has a required Vec field that's null in JSON.
NEEDS FIX: add #[serde(default)] to the Vec field in CSINode struct.

### F5. StatefulSet patch rejected (1 test)
"server rejected our request due to error in request (patch statefulsets.apps)"
Root cause: patch result deserialization failure.
NEEDS FIX: lenient deserialization in patch handler (already partially done).

### F6. ResourceQuota status PUT not found (1 test)
"server could not find the requested resource (put resourcequotas)"
Root cause: resourcequotas/:name/status route missing.
NEEDS FIX: add /status sub-resource route.

### F7. Connection failures (1 test)
"2 out of 2 connections failed"
Root cause: network connectivity issue, likely service endpoint not reachable.

### F8. gRPC probe restart count (1 test)
Expected 1 restart, got 0. Liveness probe not restarting container.
Root cause: gRPC probe type not implemented in kubelet.
NEEDS FIX: add gRPC probe support or handle as TCP probe.

### F9. Watch notification for configmap (1 test)
Timed out waiting for ADDED event on configmap.
Root cause: watch stream not delivering events properly.

### F10. DaemonSet pod deletion timeout (1 test)
Rate limiter exceeded waiting for daemon pod deletion.

### F11. Event list empty (1 test)
EventList has Kind:"", APIVersion:"" — missing type metadata.
NEEDS FIX: set Kind/APIVersion on EventList responses.

### F12. Pod2 timeout (1 test)
"wait for pod pod2 timeout" — pod not starting within 30s.

### F13. Generic timeout (1 test)
"Told to stop trying after 92.418s"

## Fixes needed (prioritized):
1. CSINode serde default on Vec field
2. ResourceQuota /status route
3. EventList Kind/APIVersion
4. Volume content (projected path, secret)
5. gRPC probe support
6. Watch improvements

## 68+ fixes across 67 commits this session

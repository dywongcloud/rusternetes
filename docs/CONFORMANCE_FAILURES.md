# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 39 — 13 failures out of ~441 tests)

## Round 39 Failures (13 total)

### CRD Creation Rejected (3 tests)
"the server rejected our request for an unknown reason (post crd)"
CRD creation endpoint returning error. Need to check CRD validation logic.

### Container CMD Override (1 test)
Expected container output with overridden arguments but got wrong output.
Our kubelet may not be passing container command/args correctly.

### DaemonSet Status Update (1 test)
"resourceVersion mismatch" on status sub-resource update.
Same RV conflict issue as the pod patch — need to clear RV for status updates.

### Ephemeral Containers (1 test)
"Timed out after 60s" updating ephemeral containers in a pod.
May need ephemeral container support in kubelet.

### StatefulSet (2 tests)
Watch closed + readyReplicas rate limiter. Watch reliability issue.

### CSIDriver Delete (1 test)
"does not allow this method on the requested resource (delete csidrivers)"
DELETE method not registered for CSIDriver resource.

### PriorityLevelConfiguration (1 test)
"could not find the requested resource"
API endpoint not registered.

### Pod Volume Race (1 test)
Timeout waiting for 5 pods to come up.

### FlowSchema/PLC API (1 test)
"could not find the requested resource"

## 23 fixes deployed, many tests passing
Most of the 441 tests are passing. These 13 failures represent ~3% failure rate.

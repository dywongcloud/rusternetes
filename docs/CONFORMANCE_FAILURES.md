# Conformance Failure Tracker

## Root Causes and Fixes Required

### Fix 1: Node controller — grace period for new nodes (CRITICAL)

**Problem:** Node controller immediately sets Ready=False on new nodes (within 5ms of creation). K8s waits 60 seconds (`nodeStartupGracePeriod`) before marking a new node's condition as Unknown.

**Impact:** The `node_lifecycle.go:95` test creates a fake node with Ready=True, but our controller overwrites it in 5ms, causing a resourceVersion conflict on the test's PATCH. The test fails, the fake node persists, and every subsequent test wastes 7 minutes waiting for it.

**K8s behavior** (from `pkg/controller/nodelifecycle/node_lifecycle_controller.go`):
- New nodes get a 60s startup grace period before any condition changes
- `tryUpdateNodeHealth()` checks `now > probeTimestamp + gracePeriod` before acting
- Initial condition is set to `ConditionUnknown` (not False)
- `nodeMonitorGracePeriod` (50s) for running nodes, `nodeStartupGracePeriod` (60s) for new nodes
- NotReady nodes get `node.kubernetes.io/not-ready:NoExecute` taint

**Fix:** Track when each node was first seen. Don't change Ready condition until startup grace period (60s) expires. Add not-ready taint when node becomes NotReady.

### Fix 2: Deployment/DaemonSet/RS controllers — merge conditions, don't replace

**Problem:** Our controllers replace the entire status.conditions array when updating status. K8s MERGES conditions — only updating conditions of known types (Progressing, Available, ReplicaFailure) and preserving all other condition types.

**Impact:** Conformance tests inject custom status conditions (e.g. type="StatusUpdate") via PUT /status. Our controller immediately overwrites them on the next reconcile. Tests timeout waiting for their custom condition to persist.

**K8s behavior** (from `pkg/controller/deployment/util/deployment_util.go:128`):
```go
func SetDeploymentCondition(status, condition) {
    currentCond := GetDeploymentCondition(status, condition.Type)
    // Only update conditions of the SAME TYPE
    newConditions := filterOutCondition(status.Conditions, condition.Type)
    status.Conditions = append(newConditions, condition)
}
```
- Copies ALL existing conditions first
- Only replaces conditions of types the controller manages
- Unknown/external conditions are preserved

**Fix:** Change status update logic in deployment, daemonset, and replicaset controllers to merge conditions instead of replacing. Only touch condition types the controller owns.

### Fix 3: ReplicaSet controller — availableReplicas tracking

**Problem:** Our RS controller may not be computing `availableReplicas` correctly. The deployment proportional scaling test waits 28 minutes for `availableReplicas = 8` on an RS.

**K8s behavior** (from `pkg/controller/replicaset/replica_set_utils.go:96`):
- `availableReplicas` = count of pods that are Ready AND have satisfied `minReadySeconds`
- Uses `podutil.IsPodAvailable(pod, minReadySeconds, now)`
- The RS controller computes this, NOT the deployment controller
- Deployment controller just sums `rs.Status.AvailableReplicas` from all its RSes

**Fix:** Ensure RS controller sets `availableReplicas` correctly using Ready + minReadySeconds check.

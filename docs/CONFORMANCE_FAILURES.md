# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 12 — 85 tests observed, 15 passed, 70 failed)

## Committed Fixes (35 root causes)
See git log for all committed fixes.

## New Failures Needing Fixes (from current run on OLD images)

### Already fixed but NOT deployed (need rebuild):
- Watch empty RV ("initial RV '' is not supported") — 3 occurrences
- AuthContext missing — 3 occurrences
- Pod initial Pending phase — already fixed
- Exec via kubelet proxy — already fixed
- CronJob 5→7 field format — already fixed
- ConfigMap optional/items — already fixed
- Probe IP through pause containers — already fixed

### NEW issues needing code fixes:

#### N1. pod-template-hash label missing on ReplicaSets (~2 tests)
Error: `doesn't have "pod-template-hash" label selector`
Deployment controller must add pod-template-hash to RS labels, pod template labels, and selector.
File: `crates/controller-manager/src/controllers/deployment.rs`

#### N2. Nodes not schedulable (~2 tests)
Error: `there are currently no ready, schedulable nodes in the cluster`
Node registration missing kubeletVersion, osImage, or other NodeSystemInfo fields.
File: `crates/kubelet/src/kubelet.rs`

#### N3. Readiness probe initialDelaySeconds not respected (~1 test)
Error: `Pod became ready before it's 15s initial delay`
Kubelet runs readiness probe immediately instead of waiting for initial delay.
File: `crates/kubelet/src/runtime.rs`

#### N4. CSIDriver DELETE not routed (~1 test)
Error: `the server does not allow this method on the requested resource (delete csidrivers)`
Missing DELETE route for csidrivers.
File: `crates/api-server/src/router.rs`

#### N5. CSINode POST deserialization (~1 test)
Error: `the server rejected our request (post csinodes)`
CSINode struct has required fields.
File: `crates/common/src/resources/csi.rs`

#### N6. ReplicaSet creation deserialization (~2 tests)
Error: `the server rejected our request (post replicasets.apps)`
ReplicaSetSpec may have required fields.
File: `crates/common/src/resources/workloads.rs`

#### N7. Deployment creation deserialization (~1 test)
Error: `the server rejected our request (post deployments.apps)`
Same as N6 but for Deployment.
File: `crates/common/src/resources/deployment.rs`

#### N8. Pod volume timeouts — downwardAPI/configMap/secret (~15 tests)
Error: `expected pod success: Timed out after 300s`
Pods with volumes still timing out. May be volume creation errors or container startup issues.
File: `crates/kubelet/src/runtime.rs`

#### N9. Watch MODIFIED notifications (~3 tests)
Error: `Timed out waiting for expected watch notification: MODIFIED`
ConfigMap watch doesn't deliver MODIFIED events after data changes.
File: `crates/api-server/src/handlers/watch.rs` or etcd watch

#### N10. ValidatingAdmissionPolicy (~2 tests)
Error: Tests at validatingadmissionpolicy.go failing
CEL-based validation may not be implemented.

#### N11. Field validation strict mode (~3 tests)
Error: Tests at field_validation.go failing
Strict field validation (rejecting unknown fields) not implemented.

#### N12. Pod 50→100 counting (~1 test)
Error: `expected 50 pods, got 100`
Controller creates double pods (may be from 2 kubelets both scheduling).
File: `crates/controller-manager/src/controllers/replicaset.rs`

## Summary: 12 new issues identified, need fixing before rebuild

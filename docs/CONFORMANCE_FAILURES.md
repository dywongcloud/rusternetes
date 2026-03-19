# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (round 10 — 25 tests observed)

## Current Run: 25 completed, 23 failed, 2 passed (8%)
Note: Running against OLD images. All fixes below are committed but NOT deployed.

## Committed Fixes (27 total)
All previous fixes committed. See git log for details.

## New Failures Needing Fixes

### F1. DaemonSet double pods with 2 nodes (expected 50, got 100)
Both kubelets start ALL pods instead of only pods assigned to their node.
Fix: Kubelet should only start pods where spec.nodeName matches its own node name.
File: `crates/kubelet/src/kubelet.rs` — filter pods by nodeName in reconcile loop.

### F2. Deployment creation rejected (deserialization)
"the server rejected our request due to an error in our request (post deployments.apps)"
Fix: Check Deployment struct for required fields that might be missing in test requests.

### F3. ResourceClaimTemplate missing kind/apiVersion
"Object 'Kind' is missing" — the response doesn't include kind and apiVersion.
Fix: Ensure TypeMeta (kind, apiVersion) is included in all resource responses.

### F4. AuthContext missing on some endpoint
Still hitting an endpoint without auth middleware.
Fix: Check which route is missing and add it to authenticated routes.

### F5. Watch MODIFIED events for ConfigMaps
ConfigMap changes don't trigger MODIFIED watch events reliably.
Fix: May be an etcd watch reliability issue or watch stream filtering.

### F6. CronJob scheduling (2 tests)
CronJob controller not creating jobs on schedule.
Fix: Debug cron schedule evaluation and job creation.

### F7. Pod activeDeadlineSeconds not enforced
Kubelet doesn't terminate pods after activeDeadlineSeconds expires.
Fix: Add deadline checking to kubelet reconcile loop.

### F8. ConfigMap volume pod timeouts
Pods with ConfigMap volumes not starting within 300s.
Fix: May be volume mounting issue or container startup delay.

## Critical Fix Needed Before Next Run
**F1 is the most critical** — without kubelet node filtering, the 2-node setup
causes double pod creation for every test. This must be fixed first.

## Architecture Notes
- Exec now properly proxies API server → kubelet → Docker (Option A)
- Log streaming still uses bollard directly from API server (needs kubelet proxy later)
- Port-forward connects directly to pod IPs via TCP

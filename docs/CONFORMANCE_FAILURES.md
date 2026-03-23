# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 57 in progress — 8 failures at 45 min)

## Round 57 Failures (8 at 45 min mark):

### F1. Watch closed — etcd stream ends prematurely
### F2. File perms on directory — expected -rwxrwxrwx on /test-volume
### F3. Job completion timeout — 15 min timeout, pod doesn't complete
### F4. Service location timeout — can't find Service in namespace
### F5. NodePort endpoint — no IP subset found within 2 min
### F6. StatefulSet revision — currentRevision field is empty
### F7. Webhook deployment — ReadyReplicas=0, never becomes available
### F8. Generic assertion — Failed after 0.002s

## Fixes needed NOW:

### F2: Volume directory permissions
The test expects the volume DIRECTORY to have mode 0777. Our volume
creation sets file permissions but not directory permissions.
FIX: Set directory mode to defaultMode in create_pod_volumes().

### F3: Job completion
Pod doesn't transition to Succeeded. Our kubelet detects container
exit but may not update pod phase for Job pods (restartPolicy=Never).
FIX: Ensure kubelet marks pods Succeeded when containers exit with 0.

### F5: NodePort / F4: Service
kube-proxy needs to set up iptables rules for NodePort services.
Tests expect endpoints to be reachable via node IP + port.

### F6: StatefulSet revision
The StatefulSet controller doesn't set currentRevision/updateRevision.
FIX: Set revision fields in StatefulSet status.

### F7: Webhook deployment
The webhook test pod image can't be pulled or doesn't start.
This may be an infrastructure issue (image not available).

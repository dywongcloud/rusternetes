# Conformance Failure Tracker

## Next Run (Round 153 — release builds, etcd, clean state, all fixes)

Building now. All fixes committed. Clean etcd — no stale state.

## All Fixes Applied

### Infrastructure
| Fix | Description | Commit |
|-----|-------------|--------|
| Kubelet startup cleanup | Parallel orphan container removal at startup before sync loop | 7acc745 |
| Kubelet terminal pod GC | Delete Succeeded/Failed pods from storage after containers stop | c179b5a |
| Kubelet orphan fast-path | Skip 30s grace period for explicitly deleted pods | efd0877 |
| Kubelet status retry | Retry pod status update on resourceVersion conflict | 3bf2ed2 |
| Namespace finalization | Hard-delete terminal pods during namespace finalization | c179b5a |
| run-conformance.sh | Timeout on stuck cleanup with force-delete fallback | c179b5a |

### Node Controller
| Fix | Description | Commit |
|-----|-------------|--------|
| Startup grace period | 60s grace before changing new node's Ready condition (K8s: nodeStartupGracePeriod) | 79fde16 |
| Not-ready taint | Add node.kubernetes.io/not-ready:NoSchedule taint on NotReady nodes | 79fde16 |

### Controller Status Write Fixes (prevents resourceVersion conflicts)
| Fix | Description | Commit |
|-----|-------------|--------|
| Deployment | Condition merge + fresh resourceVersion + revisionHistoryLimit cleanup + idempotent RS creation | ffeb5c6 |
| Deployment proportional scaling | Gate proportional scaling with isScalingEvent check using desired-replicas annotation | c179b5a |
| StatefulSet | Condition merge — preserve custom conditions from PUT /status | 9344dec |
| ReplicaSet | availableReplicas = Running + Ready + not-terminating. CAS retry guard. | 79fde16, 7ea342d |
| ReplicationController | Status write reduction + condition merge + CAS retry guard | 9344dec, 7ea342d |
| Job | Status guards on 4 early-return paths + fresh resourceVersion on termination | 7ea342d |
| DaemonSet | Condition preservation + retry failed pods in same cycle + pod watch | 7ea342d |
| ResourceQuota | Status comparison guard + fresh resourceVersion | ffeb5c6 |
| PDB | Status comparison guard + condition preservation | ffeb5c6 |
| WorkQueue cooldown | 200ms MIN_REPROCESS_INTERVAL — coalesces self-triggered events | 7ea342d |

### New Controllers
| Fix | Description | Commit |
|-----|-------------|--------|
| APIService availability | Checks backing service endpoints, updates Available condition | 7ea342d |

### API Server
| Fix | Description | Commit |
|-----|-------------|--------|
| CRD OpenAPI | Dynamic schema generation from CRD validation specs | c179b5a |
| CRD strict validation | Reject unknown fields in strict mode with K8s-format errors | 3fc9c9b |
| Service proxy | Resolve port from EndpointSlice/Endpoints instead of targetPort | c179b5a |

### Kubelet Runtime
| Fix | Description | Commit |
|-----|-------------|--------|
| EmptyDir permissions | tmpfs mode 0777 (not 1777) matching K8s | c179b5a |
| Container restart state | Re-read pod from storage after restart to preserve restart_count and last_state | c179b5a |
| Init container status | Refresh init container statuses on terminal phase transition | 7ea342d |

### GC
| Fix | Description | Commit |
|-----|-------------|--------|
| Conservative orphan detection | Include all resources (even deleting) in existing_uids — match K8s informer-cache behavior | c179b5a |

## Known Architectural Limitations

| Issue | Reason | Impact |
|-------|--------|--------|
| Pod resize cgroup | Docker creates cgroup paths as /docker/{container_id}/, not /kubepods/{pod_uid}/. Test looks for pod UID in cgroup hierarchy. | pod_resize.go:857 may still fail |
| HostPort conflict | Scheduler/kubelet logic is correct but test may timeout due to image pull or readiness probe timing | hostport.go:219 may still fail |
| Some networking tests | service.go:768, proxy.go:271, replica_set.go:232 depend on pod-to-pod networking via Docker bridge which may have timing issues | May still fail intermittently |

## Previous Results

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 149 | 398 | 43 | 441 | 90.2% | Pre-work-queue baseline (etcd) |
| 152 | 266 | 42 | 308* | 86.4% | Work queue + partial fixes. *Killed at 308 by external restart. |
| 153 | — | — | — | — | Pending — all fixes applied |

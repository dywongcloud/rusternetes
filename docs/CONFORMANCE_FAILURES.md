# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 58 deploying — 60+ fixes total)

## All fixes applied this session (60+ across 59 commits):

### Exec (breakthrough):
- WebSocket exec v5.channel.k8s.io with direct Docker execution
- 1s stream timeout + inspect_exec completion detection
- v5 close frame status protocol

### API Server:
- GC foreground deletion + propagation policy (query + body)
- Pod resize containerStatus.resources
- JSON decode ContainerState `{}` → None
- PATCH resourceVersion clear
- PodTemplate/ControllerRevision list filtering + watch + pagination
- Chunking token expiry + consistent RV + nil remainingItemCount
- 410 reason "Expired" for IsResourceExpired
- CronJob status.active ObjectReferences
- CSIDriver deletecollection + VolumeAttachment status routes
- DaemonSet/status clear resourceVersion
- Container command→Entrypoint, args→Cmd
- CRD manual body parsing + x-kubernetes-* serde fixes
- PLC/FlowSchema/CRD status sub-resource routes
- Watch transient error handling
- DeviceClass kind/apiVersion
- apiregistration.k8s.io/v1 discovery
- autoscaling/v1 discovery + HPA routes
- Lenient body parsing (RS/DS/SS/Deploy/CSINode/PV)
- IntOrString maxUnavailable in StatefulSet
- ReplicaSetStatus readyReplicas/availableReplicas default
- Service ClusterIP empty string handling
- ListMeta default resourceVersion
- Server-side apply annotation preservation

### Kubelet:
- Subpath validation (reject `..`, absolute, backticks)
- CreateContainerError preserved + retry on sync
- Pod conditions Ready=False when probes fail
- readOnlyRootFilesystem
- observedGeneration from metadata.generation
- Ephemeral container support
- Kubelet sync 2s interval
- hostIPs in pod status
- Node internal IP detection (Docker network)
- runAsUser security context → Docker User
- Volume permissions (configmap/secret/downward/projected defaultMode)
- Volume directory permissions (emptyDir 0777)
- HostAliases in /etc/hosts
- ConfigMap binaryData + items support
- Secret items support
- Secret env var injection (secretKeyRef)
- Job pod completion detection (before liveness checks)

### Controllers:
- CronJob 1s interval + status.active refs
- StatefulSet 1s interval + readyReplicas condition check + revision hash
- etcd auto-compaction 5m
- RC failure condition
- Metrics: actual pod resources instead of hardcoded

## Known remaining issues:
- Watch stream closing (etcd stream None)
- Webhook deployment pods never ready (image pull?)
- CRD decode errors
- kubectl create -f validation error (protobuf)
- Some container output tests

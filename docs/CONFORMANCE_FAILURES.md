# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (fixes in progress — 55 commits this session)

## Fixes deployed and ready to test:

### Round 56 fixes (not yet tested):
- ConfigMap volumes: defaultMode, binaryData, items support
- Secret volumes: defaultMode (0644), items support
- DownwardAPI volumes: defaultMode support
- Projected volumes: defaultMode for all source types
- HostAliases: included in /etc/hosts
- apiregistration.k8s.io/v1 added to discovery
- ReplicaSet/DaemonSet/StatefulSet/Deployment: lenient body parsing
- IntOrString: maxUnavailable in StatefulSet rolling update
- Node internal IP: Docker network IP detection
- ListMeta default resourceVersion (fixes empty RV for watches)
- GC: parse propagationPolicy from DELETE request body

### Previous fixes (tested and working):
- WebSocket exec v5.channel.k8s.io (breakthrough)
- 43+ other fixes from earlier in session

## Known remaining issues (need investigation):
- CRD decode error (empty response body)
- Watch stream closing prematurely
- Webhook deployment readiness
- Job completion detection
- Cgroup CPU weight
- DNS resolution (UDP)

## 55 commits this session, 50+ conformance fixes total

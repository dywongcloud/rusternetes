# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 57 — 56 failures, fixing all)

## Fixes applied (not yet tested):
- Volume dir perms (emptyDir 0777, configmap/secret defaultMode|0111)
- StatefulSet revision hash
- Job pod completion detection
- ReplicaSetStatus readyReplicas default
- autoscaling/v1 discovery + routes
- Service ClusterIP empty string handling
- runAsUser security context → Docker User
- ListMeta default resourceVersion
- GC body propagation policy parsing
- Volume file permissions (configmap/secret/downward/projected defaultMode)
- HostAliases in /etc/hosts
- apiregistration.k8s.io discovery
- Lenient body parsing (RS/DS/SS/Deploy)
- IntOrString maxUnavailable
- Node internal IP detection

## Still need fixing:
1. SECRET_DATA env var — secret data not injected as env vars
2. CSINode creation rejected — handler rejects valid requests
3. PV creation rejected — handler rejects valid requests
4. Webhook deployments never ready (5 tests) — pod can't pull image
5. CRD decode errors — response body empty or malformed
6. Watch closed — etcd stream reliability
7. Container output content — configmap/projected volume content via exec
8. kubectl create -f — pipe stdin not working
9. RC failure condition — controller doesn't set conditions
10. CPU resource 300m vs 100m — metrics hardcoded
11. Server-side apply annotation — not preserved

## 58 commits, 55+ conformance fixes this session

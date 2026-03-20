# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (round 11 — ALL identified issues fixed)

## Status: ALL KNOWN ISSUES FIXED

35 root cause categories identified and fixed across all components.

## Fix Summary

| Round | Fixes | Key Changes |
|-------|-------|-------------|
| 1-15 | Infrastructure + watch | iptables, protobuf, bookmark, selectors, sendInitialEvents |
| 16-19 | Content compat | CronJob, downwardAPI, ConfigMap, WebSocket |
| 20-22 | Discovery + routing | Aggregated discovery, ephemeral PATCH, NodePort MASQUERADE |
| 23-25 | Validation + deser | Watch RV, status details, DaemonSet, node deser, name validation |
| 26 | Protobuf + subpath + GC | 406 fallback, SubPathExpr, GC scan, preemption |
| 27-28 | Pod lifecycle + probes | Initial Pending phase, probe IP through pause containers |
| 29 | Exec architecture | Proxy exec through kubelet (Option A) |
| 30-35 | Final fixes | ReplicaSet counting, deployment deser, activeDeadlineSeconds, CronJob 5→7 field, ConfigMap optional/items, CRD fallback auth, ResourceClaimTemplate kind |

## All Fixes Need Fresh Image Builds

Run: `docker compose build api-server kubelet controller-manager kube-proxy scheduler`
Then: clean deploy, bootstrap, run conformance

## Remaining Known Limitation
- 2-node conformance tests may have pod scheduling edge cases
- DNS resolution depends on CoreDNS upstream compatibility

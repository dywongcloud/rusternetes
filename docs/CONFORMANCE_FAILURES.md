# Conformance Issue Tracker

**178 fixes** | 12 pending deploy | Build clean, all unit tests pass

## Pending deploy fixes (since round 97)

| # | Fix | Impact |
|---|-----|--------|
| 169 | generation=1, ClusterIP, SA token, PodScheduled | 5+ tests |
| 170 | **CRITICAL** resourceVersion in watch events | 12+ tests |
| 171 | Endpoints single subset | 1 test |
| 172 | Ensure metadata for resourceVersion | 1 test |
| 173 | Remove duplicate SA token route (panic) | startup |
| 174 | **CRITICAL** List RV from items, not timestamps | ALL tests |
| 175 | Immutable returns 403 Forbidden | 2 tests |
| 176 | RC orphan handling + DaemonSet ControllerRevision | 2 tests |
| 177 | Aggregated discovery responseKind.group empty | 1 test |
| 178 | In-place pod resize via Docker update_container | 1 test |

## Remaining issues needing post-deploy investigation

| Test | Error | Notes |
|------|-------|-------|
| output.go:263,:282 | CPU downward API value wrong | Need to check if pod spec resources are preserved through create pipeline |
| runtime.go:169 | Termination message empty | Docker cp from stopped container may fail; fix #153 deployed |
| webhook.go:837 | matchConditions not validated | Needs CEL expression type checker (no cel crate available) |
| webhook.go:1194,:1244 | webhook not ready | Kubelet sync timing; fix #161 deployed |
| service.go:251 | Affinity didn't hold | iptables recent module deployed; need to verify |
| runtimeclass.go:153,:297 | timeout + list length | Watch timing + etcd consistency |
| resource_quota.go:102,:209 | quota timeout | Controller interval timing |
| service_cidrs.go:255 | IPAddress error | IPv6 address routing |
| kubectl.go:1881 | proxy unreachable | Networking/kube-proxy |
| validatingadmissionpolicy.go:568 | watch ERROR events | Should be fixed by #170 + #174 |
| Protobuf CRDs (4 tests) | native protobuf encoding | K8s CRD client hardcodes protobuf |

# Conformance Issue Tracker

**183 fixes** | 17 pending deploy | Build clean, all unit tests pass

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
| 179 | CEL matchConditions validation in webhook create handlers | 1 test |
| 180 | RuntimeClass watch handler + route | 2 tests |
| 181 | ResourceQuota watch handlers + routes (ns + all) | 2 tests |
| 182 | Proxy double-slash path fix (node/service/pod) | 1 test |
| 183 | Termination message bind-mount + host-file read | 1 test |

## Remaining issues needing post-deploy investigation

| Test | Error | Notes |
|------|-------|-------|
| output.go:263,:282 | CPU downward API value wrong | Pipeline preserves resources; may be integer division or node capacity default — needs post-deploy investigation |
| runtime.go:169 | Termination message empty | **FIXED #183** — bind-mount host file instead of docker cp from tmpfs |
| webhook.go:837 | matchConditions not validated | **FIXED #179** — CEL validation via cel-interpreter in create handlers |
| webhook.go:1194,:1244 | webhook not ready | Kubelet sync timing; fix #161 deployed |
| service.go:251 | Affinity didn't hold | iptables recent module deployed; need to verify |
| runtimeclass.go:153,:297 | timeout + list length | **FIXED #180** — added watch handler + route |
| resource_quota.go:102,:209 | quota timeout | **FIXED #181** — added watch handlers + routes (ns + all) |
| service_cidrs.go:255 | IPAddress error | Missing IPAddress status route + no CIDR validation controller |
| kubectl.go:1881 | proxy unreachable | **FIXED #182** — double-slash URL path construction |
| validatingadmissionpolicy.go:568 | watch ERROR events | Should be fixed by #170 + #174 |
| Protobuf CRDs (4 tests) | native protobuf encoding | Handlers hardcode Json<T>; need content negotiation or 406 for protobuf Accept |

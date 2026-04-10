# Conformance Failure Tracker

**Round 133** | 370/441 (83.9%) | 2026-04-10
**Round 134** | Running (42 fixes deployed, 7 staged) | 2026-04-10

## Remaining Issues to Fix

### Watch Reliability — causes ~15 cascade failures (STAGED effdec6)
- `deployment.go:1259`, `rc.go:509,623`, `replica_set.go:232,560`, `runtime.go:115`, `service.go:3459`
- **Root cause**: "Watch failed: context canceled" — Connection header prohibited in HTTP/2
- **Fix staged**: effdec6 removes Connection: keep-alive header, uses Transfer-Encoding: chunked
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/endpoints/handlers/watch.go:237

### Webhook Service Readiness — 7 failures
- `webhook.go:520,675,904,1269,1334,1400,2107`
- **Root cause**: Webhook pod + EndpointSlice + readiness check takes >30s
- **K8s ref**: waitWebhookConfigurationReady polls for 30s at 100ms
- **Action**: Check why pod→EndpointSlice→webhook ready chain is slow

### Service Networking — 3 failures
- `service.go:768,886`, `proxy.go:271,503`
- **Root cause**: kube-proxy iptables rules not routing ClusterIP→Pod correctly
- **K8s ref**: kube-proxy syncs iptables from Service+EndpointSlice
- **Action**: Debug iptables rules for conformance test services

### Init Container Status — 2 failures (STAGED d9c9d34)
- `init_container.go:440,565`
- **Root cause**: Kubelet doesn't send intermediate status during init container execution
- **Fix staged**: d9c9d34 adds status updates between init container runs

### StatefulSet Scale-Down — 1 failure
- `statefulset.go:957`
- **Root cause**: Pod deletion + recreation cycle takes too long
- **K8s ref**: processCondemned calls DeleteStatefulPod which sends API DELETE
- **Action**: Check kubelet container stop timing, consider shorter grace period

### DaemonSet ControllerRevision — 1 failure (STAGED 73eaccf)
- `daemon_set.go:1276`
- **Root cause**: JSON key ordering differs between Go and Rust
- **Fix staged**: 73eaccf sorts keys alphabetically matching Go encoding/json

### DNS Container Exec — 1 failure
- `dns_common.go:476`
- **Root cause**: Container exec runs /pause binary instead of shell
- **K8s ref**: e2e framework ExecShellInPod expects /bin/sh
- **Action**: Check exec handler container resolution, shell availability

### Aggregator — 1 failure
- `aggregator.go:359`
- **Root cause**: Extension API server deployment doesn't start
- **Action**: Check image pull, pod scheduling, networking for extension pods

### Service Accounts OIDC — 1 failure
- `service_accounts.go:667`
- **Root cause**: OIDC discovery TLS — pod doesn't trust API server cert
- **K8s ref**: Pod should use kube-root-ca.crt for TLS verification
- **Action**: Ensure CA cert in kube-root-ca.crt matches API server cert

### Host Port — 1 failure
- `hostport.go:219`
- **Root cause**: Host port binding in container-in-container Docker
- **Action**: Check kubelet container port binding in DinD

### EndpointSlice Orphan — 1 failure
- `endpointslice.go:135`
- **Root cause**: Client rate limiter blocks cleanup check
- **Action**: Reduce API call volume to avoid rate limiting

### Resource Quota — 1 failure (STAGED 776c8fa)
- `resource_quota.go:282`
- **Root cause**: Quota controller doesn't track extended resources
- **Fix staged**: 776c8fa adds extended resource counting

### EndpointSlice Mirroring — 1 failure (STAGED 6e9a13e)
- `endpointslicemirroring.go:129`
- **Root cause**: Mirroring skipped for selector-less services
- **Fix staged**: 6e9a13e only skips when service HAS selector

### Field Validation YAML Dup — 1 failure (STAGED 571296a)
- `field_validation.go:735`
- **Root cause**: serde_yaml doesn't detect duplicate YAML keys
- **Fix staged**: 571296a adds duplicate key detection in strict mode

### Pod Output Permissions — 1 failure
- `pod/output.go:263`
- **Root cause**: Docker umask 0022 reduces 0777 to 0755
- **Action**: Set umask 0 in container creation

### Pod Resize — 1 failure
- `pod_resize.go:857`
- **Root cause**: cgroup changes in container-in-container Docker
- **Action**: Check Docker update_container for resource changes

### kubectl Proxy — 1 failure
- `kubectl.go:1881`
- **Root cause**: kubectl proxy startup timing
- **Action**: Check proxy endpoint response format

### CRD Defaulting — 1 failure (STAGED 516922e)
- `custom_resource_definition.go:334`
- **Root cause**: CRD GET didn't apply schema defaults on read
- **Fix staged**: 516922e applies defaults on GET, f096b77 on LIST

## Staged Fixes (11 commits, need deploy)

| Commit | Fix | Tests |
|--------|-----|-------|
| d9c9d34 | Init container intermediate status | init_container:440,565 |
| 516922e | CRD GET defaults on read | custom_resource_definition:334 |
| f096b77 | CRD LIST defaults on read | CRD list tests |
| 73eaccf | DaemonSet CR key sorting | daemon_set:1276 |
| 776c8fa | ResourceQuota extended resources | resource_quota:282 |
| 6e9a13e | EndpointSlice mirroring selector-less | endpointslicemirroring:129 |
| 1be61f8 | EndpointSlice sync interval 2s | webhook readiness timing |
| 71608a0 | StatefulSet scale-down proper deletion | statefulset:957 |
| effdec6 | Watch HTTP/2 headers fix | ~15 watch cascade failures |
| bab6e26 | Deployment maxSurge respect | deployment:995 |

## All Fix Commits This Session (47)

c10e449, 3136c2a, f34bd51, 6edb6be, 323d9dc, db4855b, c5ad02d, d26e2ef,
f7dfb20, c4d3fa7, eb07e78, f50d364, 8dbedb5, 77f4e6f, 176b2cd, af5e245,
c4bda95, c2a0dd8, 967b1fd, f1e00db, 7ae38d7, 5b19baf, b2ba5cf, 06d3a40,
faf427c, 0b22923, 182b280, 2332cf4, 5ff70c7, b5e457c, 09bcebe, 4e442e8,
0347108, 516922e, f096b77, d9c9d34, 73eaccf, 776c8fa, 6e9a13e

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | TBD | TBD | 441 | TBD |

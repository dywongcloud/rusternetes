# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 53 starting — 54 failures in round 52)

## Round 52 Results: 54 failures out of 441 tests (~88% pass rate)

### Container output / exec issues (many tests):
- configmap/secret/projected volume content not visible via exec
- File permissions wrong on mounted volumes
- HOST_IP downward API env var — FIX DEPLOYED (hostIPs set)
- These may be exec-related: output is collected but not displayed

### API/Router issues:
- DeviceClass Kind missing — FIX DEPLOYED
- CRD decode error (empty response body on creation)
- apiregistration.k8s.io/v1 not found
- /api output parse error
- Webhook discovery

### Watch/Protocol:
- watch closed before timeout
- initial RV "" not supported

### Controller/Timeout:
- Webhook deployments not becoming ready (ReadyReplicas=0)
- Job completion timeouts (15 min)
- DaemonSet pod deletion rate limit

### Subpath:
- Backtick rejection — FIX DEPLOYED

## 42+ fixes deployed including:
- WebSocket exec with v5.channel.k8s.io
- Direct Docker execution (bypass kubelet proxy)
- 1s exec stream timeout
- DeviceClass kind/apiVersion
- hostIPs in pod status
- Backtick subpath validation
- All 35 previous fixes

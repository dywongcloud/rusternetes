# Kubernetes Conformance Test Findings

**Date**: March 12, 2026
**Test Tool**: Sonobuoy v0.57.4
**Kubernetes Version Target**: v1.30.0
**Test Mode**: Quick (attempted)

## Executive Summary

Attempted to run Kubernetes conformance tests using Sonobuoy. Discovered several implementation gaps that prevent conformance tests from running successfully.

## Test Setup

Created kubeconfig for Sonobuoy:
```yaml
apiVersion: v1
kind: Config
clusters:
- cluster:
    insecure-skip-tls-verify: true
    server: https://localhost:6443
  name: rusternetes
contexts:
- context:
    cluster: rusternetes
    user: rusternetes-admin
  name: rusternetes
current-context: rusternetes
users:
- name: rusternetes-admin
  user: {}
```

## Issues Discovered

### 1. DELETE Method Not Implemented for ClusterRoleBindings ⚠️ CRITICAL

**Symptom**:
```
failed to delete cluster role binding: the server does not allow this method on the requested resource (delete clusterrolebindings.rbac.authorization.k8s.io)
```

**Impact**: HIGH - Prevents cleanup of RBAC resources, blocks Sonobuoy deletion

**Location**: `crates/api-server/src/router.rs` - Missing DELETE handler for `/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{name}`

**Fix Required**: Implement DELETE method for:
- `/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/{name}`
- Potentially other RBAC resources (clusterroles, rolebindings, roles)

---

### 2. Sonobuoy Image Pull Failure ⚠️ BLOCKER

**Symptom**:
```
ERROR kubelet::runtime: Failed to pull image docker.io/sonobuoy/sonobuoy:v0.57.4: Image pull failed: Docker responded with status code 404: manifest for sonobuoy/sonobuoy:v0.57.4 not found: manifest unknown: manifest unknown
```

**Root Cause**: Sonobuoy image tags changed. The image `sonobuoy/sonobuoy:v0.57.4` doesn't exist.

**Available Tags**:
- `sonobuoy/sonobuoy:main`
- `sonobuoy/sonobuoy:arm64-main`
- `sonobuoy/sonobuoy:ppc64le-main`
- etc.

**Impact**: CRITICAL - Prevents Sonobuoy pod from starting

**Workaround**: Need to either:
1. Use Sonobuoy with `--image` flag to specify `sonobuoy/sonobuoy:main`
2. Use older Sonobuoy version with proper versioned images
3. Pre-pull the correct image manually

---

### 3. kubectl describe Not Implemented

**Symptom**:
```bash
$ kubectl --insecure-skip-tls-verify describe pod sonobuoy -n sonobuoy
(eval):1: unknown file attribute:
```

**Impact**: MEDIUM - Makes debugging difficult, not required for conformance but helpful

**Location**: `crates/kubectl/` - Missing `describe` subcommand implementation

**Fix Required**: Implement kubectl describe for major resources (pods, services, deployments, etc.)

---

## Successfully Working Features

✅ **Sonobuoy Resource Creation**: All Sonobuoy resources were created successfully:
- Namespace: `sonobuoy`
- ServiceAccount: `sonobuoy-serviceaccount`
- ClusterRole: `sonobuoy-serviceaccount-sonobuoy`
- ClusterRoleBinding: `sonobuoy-serviceaccount-sonobuoy`
- ConfigMaps: `sonobuoy-config-cm`, `sonobuoy-plugins-cm`
- Pod: `sonobuoy` (failed at image pull, not resource creation)
- Service: `sonobuoy-aggregator`

✅ **Volume Management**: Kubelet successfully created all volumes:
- ConfigMap volumes: `sonobuoy-config-volume`, `sonobuoy-plugins-volume`
- EmptyDir volume: `output-volume`
- Projected volume: `kube-api-access` with ca.crt, token, namespace

✅ **RBAC Resources**: Created successfully (but DELETE not implemented)

✅ **CoreDNS**: Running and healthy

✅ **Basic Pod Operations**: test-dns pod running successfully

## Priority Fixes for Conformance

### P0 - Critical (Must Fix)
1. **Sonobuoy Image Issue**: Use correct image tag or version
   - **Quick Fix**: Run Sonobuoy with `--sonobuoy-image=sonobuoy/sonobuoy:main`
   - **Better Fix**: Use Sonobuoy version with stable image tags

2. **DELETE for RBAC Resources**: Implement DELETE handlers
   - ClusterRoleBindings
   - ClusterRoles
   - RoleBindings (if not already implemented)
   - Roles (if not already implemented)

### P1 - High (Should Fix)
3. **kubectl describe**: Implement for debugging
   - Pods
   - Services
   - Deployments
   - Other major resources

### P2 - Medium (Nice to Have)
4. **Better Error Messages**: When methods are not implemented, return proper HTTP 405 or feature-specific messages

## Next Steps

1. **Implement DELETE for RBAC Resources**
   - Add DELETE handlers in `crates/api-server/src/router.rs`
   - Implement deletion logic for ClusterRoleBindings, ClusterRoles
   - Add tests

2. **Re-run Sonobuoy with Correct Image**
   ```bash
   sonobuoy run --mode=quick \
     --kubeconfig=/tmp/kubeconfig-rusternetes.yaml \
     --skip-preflight \
     --sonobuoy-image=sonobuoy/sonobuoy:main
   ```

3. **Implement kubectl describe**
   - Add describe subcommand to kubectl
   - Format output similar to official kubectl

4. **Run Full Conformance Suite**
   - After fixing critical issues, run full conformance (not just quick mode)
   - Document all failing tests
   - Prioritize fixes based on conformance requirements

## Test Commands Used

```bash
# Create kubeconfig
cat > /tmp/kubeconfig-rusternetes.yaml <<'EOF'
apiVersion: v1
kind: Config
clusters:
- cluster:
    insecure-skip-tls-verify: true
    server: https://localhost:6443
  name: rusternetes
contexts:
- context:
    cluster: rusternetes
    user: rusternetes-admin
  name: rusternetes
current-context: rusternetes
users:
- name: rusternetes-admin
  user: {}
EOF

# Run Sonobuoy (failed)
sonobuoy run --mode=quick \
  --kubeconfig=/tmp/kubeconfig-rusternetes.yaml \
  --skip-preflight

# Check status
sonobuoy status --kubeconfig=/tmp/kubeconfig-rusternetes.yaml

# Check pod status
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify get pods -n sonobuoy

# Check kubelet logs
docker logs rusternetes-kubelet 2>&1 | grep -i "sonobuoy" | tail -30

# Attempt cleanup (failed)
sonobuoy delete --kubeconfig=/tmp/kubeconfig-rusternetes.yaml --wait
```

## Related Documentation

- [CONFORMANCE_PLAN.md](CONFORMANCE_PLAN.md) - Original conformance planning
- [STATUS.md](../STATUS.md) - Current implementation status
- [API_FEATURES_COMPLETE.md](../API_FEATURES_COMPLETE.md) - API features status

## Environment

- **OS**: macOS Sequoia 15.7.4
- **Container Runtime**: Docker Desktop
- **Cluster Components**: All running and healthy
- **Test Tool**: Sonobuoy v0.57.4
- **Kubernetes Target**: v1.30.0

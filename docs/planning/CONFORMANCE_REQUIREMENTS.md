# Kubernetes Conformance Requirements for Rusternetes

Based on the Sonobuoy conformance test run on 2026-03-13, the following features need to be implemented for Rusternetes to pass Kubernetes conformance tests.

## Infrastructure (✅ COMPLETED)

- ✅ TLS certificates with proper SANs including `kubernetes.default.svc.cluster.local`
- ✅ DNS resolution working (CoreDNS configured)
- ✅ ClusterIP routing configured (kubernetes service at 10.96.0.1)
- ✅ Certificate auto-copy to CoreDNS volume

## Critical Issues

### 1. HTTP Method Support
**Status**: ✅ WORKING - All methods including DELETE work correctly
**Verification**: Tested via curl - DaemonSet and Job CREATE/DELETE work
**Note**: Sonobuoy error messages were misleading client-side interpretations

### 2. Core Resources Status
**Status**: ✅ IMPLEMENTED AND WORKING
**Verified working via direct API calls**:
- DaemonSets (apps/v1) - CREATE ✅ DELETE ✅
- Jobs (batch/v1) - CREATE ✅
- Pods (v1) - All operations working ✅

**Note**: The sonobuoy errors about "could not create DaemonSet" and "could not create Job" were NOT due to missing resources. These resources are fully implemented.

### 3. Actual Root Cause: Missing API Groups
**Status**: BLOCKING - This is the real issue
**Error**: `unable to retrieve the complete list of server APIs: authentication.k8s.io/v1: the server could not find the requested resource`
**Impact**: Sonobuoy's API discovery fails, causing it to not find available endpoints

**Root cause analysis**:
- Sonobuoy uses API discovery to find available resources
- When API groups are missing from discovery, sonobuoy can't find the endpoints
- This causes misleading error messages about resources not existing
- The resources EXIST and WORK, but sonobuoy can't discover them

### 4. Missing API Groups

The following API groups are required but missing:

#### authentication.k8s.io/v1
- TokenReview resource
- Required for service account token validation

#### authorization.k8s.io/v1
- SubjectAccessReview resource
- SelfSubjectAccessReview resource
- Required for RBAC checks

#### autoscaling/v2
- HorizontalPodAutoscaler resource
- Required for auto-scaling (may be optional for basic conformance)

#### metrics.k8s.io/v1beta1
- Node metrics
- Pod metrics
- Required by HPA and monitoring tools

#### node.k8s.io/v1
- RuntimeClass resource
- Required for advanced pod scheduling

#### policy/v1
- PodDisruptionBudget resource
- Required for cluster maintenance and updates

#### resource.k8s.io/v1
- ResourceClaim resource
- Required for dynamic resource allocation

## Discovery API Issues

**Error**: `unable to retrieve the complete list of server APIs`
**Current Status**: `/apis` endpoint exists but may not return all required API groups
**Requirement**: Ensure ALL API groups are properly advertised in discovery
**Format**: APIGroupList resource must include all groups (even if they return empty resource lists)

## Recommendations - UPDATED BASED ON FINDINGS

### Phase 1: Fix API Discovery (CRITICAL - DO THIS FIRST)
1. ✅ Core resources are already implemented (DaemonSet, Job work!)
2. ✅ DELETE methods work correctly
3. ❌ FIX: Ensure `/apis` endpoint returns ALL API groups in APIGroupList
4. ❌ FIX: Implement stub endpoints for missing API groups so discovery doesn't fail
5. Strategy: Add minimal implementations that return empty lists rather than 404

### Phase 2: Add Stub API Groups (Required for Sonobuoy discovery)
Add minimal stub implementations for:
1. authentication.k8s.io/v1 (TokenReview) - Return empty/not implemented
2. authorization.k8s.io/v1 (SubjectAccessReview) - Return empty/not implemented
3. policy/v1 (PodDisruptionBudget) - May already exist, verify
4. node.k8s.io/v1 (RuntimeClass) - Return empty
5. metrics.k8s.io/v1beta1 - Can return "not available"
6. autoscaling/v2 - May already exist as we have HPA
7. resource.k8s.io/v1 - Return empty

### Phase 3: Full Implementations (After basic conformance passes)
1. Proper TokenReview implementation
2. Proper SubjectAccessReview implementation
3. RuntimeClass support
4. Metrics server integration

## Test Results Summary

- **DNS**: ✅ Working (kubernetes.default.svc.cluster.local resolves)
- **TLS**: ✅ Working (no certificate errors)
- **Core Resources**: ✅ Working (DaemonSet, Job, Pod CREATE/DELETE verified)
- **API Discovery**: ❌ FAILING - Missing API groups block sonobuoy discovery
- **Plugin Launch**: ❌ Failed due to API discovery issues (NOT missing resources)
- **Test Execution**: ❌ Did not run (API discovery blocked plugin launch)
- **Overall Status**: **NOT CONFORMANT** - API discovery incomplete

## Next Steps - UPDATED

1. ✅ DONE: Verified DaemonSet and Job work correctly
2. ✅ DONE: Verified DELETE methods work
3. ❌ TODO: Check current `/apis` endpoint implementation
4. ❌ TODO: Add missing API groups to discovery response
5. ❌ TODO: Add stub handlers for missing API groups
6. ❌ TODO: Re-run conformance tests to verify discovery works

## Files Modified in This Session

- `bootstrap-cluster.yaml` - Added clusterIP: "10.96.0.1" to kubernetes service
- `scripts/generate-certs.sh` - Added kubernetes SANs and auto-copy to CoreDNS volume
- `.rusternetes/certs/api-server.crt` - Regenerated with new SANs

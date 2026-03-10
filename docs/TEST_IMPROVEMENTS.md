# Test Improvements Roadmap

This document tracks suggested unit and integration tests to improve test coverage for Rusternetes.

**Last Updated**: March 10, 2026

## Implementation Progress

**Completed**: 3/10 test suites ✅
**In Progress**: 0/10 test suites 🟡
**Total New Tests Added**: 23 passing tests

### Recently Completed (March 10, 2026)

1. ✅ **PDB Controller Tests** - 7 comprehensive tests covering:
   - Disruption prevention logic
   - Eviction blocking when disruptions_allowed=0
   - Label selector matching
   - Namespace isolation
   - Percentage-based values (80%, 30%)
   - Status with conditions
   - Multi-namespace listing

   **Location**: `crates/controller-manager/tests/pdb_controller_test.rs`
   **Result**: All 7 tests passing ✅

2. ✅ **Init Containers E2E Tests** - 7 comprehensive tests covering:
   - Pod structure with init containers
   - Sequential execution verification
   - Status transitions (Pending → Running → Completed)
   - Init container failure blocking app containers
   - Restart count tracking
   - Multiple init containers sequential execution
   - Serialization/deserialization

   **Location**: `crates/kubelet/tests/init_containers_test.rs`
   **Result**: All 7 tests passing ✅

3. ✅ **CNI Integration Tests** - 9 comprehensive tests covering:
   - Plugin discovery and management
   - ADD operation execution
   - DEL operation and cleanup
   - Network config validation
   - Config loading from files
   - Multiple network attachments
   - Error handling with missing plugins
   - CNI result parsing
   - Plugin chaining with conflist

   **Location**: `crates/kubelet/tests/cni_integration_test.rs`
   **Fixtures**: `crates/kubelet/tests/fixtures/mock-cni-plugin.sh` and config files
   **Result**: All 9 tests passing ✅

## Test Priority Matrix

| Test Category | Priority | Reason | Estimated Effort | Status |
|--------------|----------|--------|------------------|--------|
| CNI Integration | **High** | Critical for networking, no integration tests | Medium | ✅ **Complete** (9 tests) |
| Leader Election | **High** | Critical for HA, only 1 ignored test | Medium | ⏹️ Not Started |
| HPA/VPA Integration | **Medium** | New feature, needs validation | High | ⏹️ Not Started |
| PDB Integration | **Medium** | No tests at all | Low | ✅ **Complete** (7 tests) |
| Init Containers E2E | **Medium** | Common use case, needs runtime test | Low | ✅ **Complete** (7 tests) |
| Admission Webhooks E2E | **Medium** | Complex feature, needs real webhook | Medium | ⏹️ Not Started |
| HA Cluster | **Medium** | Important for production | High | ⏹️ Not Started |
| Resource Lifecycle | **Low** | Good coverage already | Low | ⏹️ Not Started |
| DNS Integration Expansion | **Low** | 15 tests already exist | Low | ⏹️ Not Started |
| LoadBalancer Integration | **Low** | 16 tests already exist | Low | ⏹️ Not Started |

## Quick Wins (Low Effort, High Value)

1. ✅ **PDB Controller Tests** - Simple CRUD logic, easy to test
2. ✅ **Init Containers Runtime Tests** - Single pod flow, straightforward
3. ✅ **CNI Mock Plugin Tests** - Can use shell script as mock plugin

---

## 1. CNI Integration Tests (High Priority) ✅ COMPLETE

**Current Status**: ✅ 9 integration tests passing (16 unit tests already existed)
**Location**: `crates/kubelet/tests/cni_integration_test.rs`
**Fixtures**: `crates/kubelet/tests/fixtures/mock-cni-plugin.sh`, `test-network.conf`, `test-network-chain.conflist`
**Estimated Effort**: Medium
**Completed**: March 10, 2026

### Test Cases

#### 1.1 Plugin Discovery ✅
- **Description**: Test plugin discovery and management
- **Implementation**: ✅ `test_cni_plugin_discovery` - Passing
- **Coverage**: Plugin discovery from custom paths, executable validation, list_plugins API

#### 1.2 ADD Operation Execution ✅
- **Description**: Test actual ADD operation with a mock CNI plugin
- **Implementation**: ✅ `test_cni_plugin_execution_add` - Passing
- **Coverage**: Network setup, CNI result parsing, IP address assignment, mock plugin execution

#### 1.3 DEL Operation and Cleanup ✅
- **Description**: Test DEL operation and network teardown
- **Implementation**: ✅ `test_cni_plugin_execution_del` - Passing
- **Coverage**: Network setup followed by teardown, cleanup verification

#### 1.4 Network Config Validation ✅
- **Description**: Test network configuration parsing and validation
- **Implementation**: ✅ `test_cni_network_config_validation` - Passing
- **Coverage**: JSON parsing, config structure validation, required fields

#### 1.5 Config Loading ✅
- **Description**: Test loading configurations from file system
- **Implementation**: ✅ `test_cni_config_loading` - Passing
- **Coverage**: Config directory scanning, .conf file loading

#### 1.6 Multiple Network Attachments ✅
- **Description**: Test pods with multiple network attachments
- **Implementation**: ✅ `test_cni_multiple_attachments` - Passing
- **Coverage**: Multiple ADD operations, attachment tracking, stats collection

#### 1.7 Error Handling ✅
- **Description**: Test CNI error scenarios
- **Implementation**: ✅ `test_cni_error_handling_missing_plugin` - Passing
- **Coverage**: Missing plugin errors, proper error propagation

#### 1.8 CNI Result Parsing ✅
- **Description**: Test CNI result format parsing
- **Implementation**: ✅ `test_cni_result_parsing` - Passing
- **Coverage**: JSON deserialization, interfaces/IPs/routes/DNS parsing, primary_ip extraction

#### 1.9 Plugin Chaining ✅
- **Description**: Test .conflist files with multiple plugins
- **Implementation**: ✅ `test_cni_plugin_chaining` - Passing
- **Coverage**: Conflist loading, plugin chain execution

**Summary**: All 9 CNI integration test scenarios implemented and passing. Tests cover plugin discovery, ADD/DEL operations, config management, error handling, and plugin chaining. Mock plugin fixture created for testing without requiring actual CNI binaries.

---

## 2. Leader Election Integration Tests (High Priority)

**Current Status**: 1 ignored test requiring etcd
**Location**: `crates/common/tests/leader_election_test.rs` (to be created)
**Estimated Effort**: Medium

### Test Cases

#### 2.1 Failover Testing
- **Description**: Kill the leader and verify follower takes over
- **Setup**: Start 2 LeaderElector instances
- **Test**:
  - Wait for leader election
  - Verify one is leader, one is follower
  - Shutdown leader instance
  - Verify follower becomes leader within ~15 seconds
  - Verify no split-brain (both claim leadership)

#### 2.2 Split-Brain Prevention
- **Description**: Test network partition scenarios
- **Setup**: Run 3 instances with etcd cluster
- **Test**:
  - Partition network (isolate 1 node)
  - Verify isolated node loses leadership
  - Verify other 2 nodes elect new leader
  - Heal partition
  - Verify only 1 leader after reconciliation

#### 2.3 Lease Expiration
- **Description**: Test what happens when lease isn't renewed
- **Setup**: Leader with lease_duration=5s, renew_interval=10s (intentionally wrong)
- **Test**:
  - Verify leader acquires leadership
  - Wait for lease to expire
  - Verify leader detects lost leadership
  - Verify follower claims leadership

#### 2.4 Multiple Concurrent Elections
- **Description**: Test with 3+ instances
- **Setup**: Start 5 LeaderElector instances simultaneously
- **Test**:
  - All instances compete for leadership
  - Verify exactly 1 becomes leader
  - Verify 4 become followers
  - Kill leader, verify one of 4 takes over

#### 2.5 Graceful Shutdown
- **Description**: Test leadership release on shutdown
- **Setup**: Leader instance
- **Test**:
  - Call `shutdown()` on leader
  - Verify leadership is released (key deleted in etcd)
  - Verify lease is revoked
  - Verify follower can immediately acquire leadership

#### 2.6 Lease Renewal
- **Description**: Test long-running leader maintains lease
- **Setup**: Leader with short lease (5s), long runtime (60s)
- **Test**:
  - Leader runs for 60 seconds
  - Verify lease is renewed at least 10 times
  - Verify leadership never lost
  - Verify no errors in renewal

---

## 3. HPA/VPA Controller Integration Tests (Medium Priority)

**Current Status**: 3 unit tests for HPA, 0 for VPA
**Location**: `crates/controller-manager/tests/hpa_controller_test.rs`, `crates/controller-manager/tests/vpa_controller_test.rs`
**Estimated Effort**: High

### HPA Test Cases

#### 3.1 HPA Scaling Workflow
- **Description**: End-to-end HPA scaling test
- **Setup**: Deployment with 2 replicas, HPA with target CPU 50%
- **Test**:
  - Create deployment
  - Create HPA pointing to deployment
  - Mock metrics showing 80% CPU usage
  - Verify HPA scales up deployment
  - Mock metrics showing 20% CPU usage
  - Verify HPA scales down deployment

#### 3.2 Scale Limits Enforcement
- **Description**: Test min/max replica enforcement
- **Setup**: HPA with minReplicas=2, maxReplicas=10
- **Test**:
  - Deployment has 1 replica - verify scaled to 2
  - High CPU load triggers scale to 15 - verify capped at 10
  - Low CPU load triggers scale to 1 - verify floored at 2

#### 3.3 Cooldown Periods
- **Description**: Test scaling behavior configuration
- **Setup**: HPA with scaleDown stabilizationWindowSeconds=300
- **Test**:
  - Trigger scale down
  - Verify scale down doesn't happen immediately
  - Wait 5 minutes
  - Verify scale down occurs

#### 3.4 Multiple Metrics
- **Description**: Test HPA with multiple metric sources
- **Setup**: HPA with CPU and memory targets
- **Test**:
  - CPU at 80%, memory at 40% - verify scale decision
  - CPU at 40%, memory at 80% - verify scale decision
  - Both at 80% - verify aggressive scaling

### VPA Test Cases

#### 3.5 VPA Recommendation Workflow
- **Description**: Test VPA generates recommendations
- **Setup**: Deployment with containers requesting 100m CPU, VPA in "Off" mode
- **Test**:
  - Pods actually using 500m CPU
  - VPA analyzes usage
  - Verify VPA status contains recommendations for 500m+ CPU

#### 3.6 VPA Update Modes
- **Description**: Test different VPA update modes
- **Test Cases**:
  - **Off**: Recommendations only, no pod updates
  - **Initial**: Resources set only on pod creation
  - **Recreate**: Pods deleted and recreated with new resources
  - **Auto**: Automatic updates (if implemented)

---

## 4. PDB Integration Tests (Medium Priority) ✅ COMPLETE

**Current Status**: ✅ 7 tests passing
**Location**: `crates/controller-manager/tests/pdb_controller_test.rs`
**Estimated Effort**: Low
**Completed**: March 10, 2026

### Test Cases

#### 4.1 Disruption Prevention ✅
- **Description**: Test PDB blocks pod evictions
- **Setup**: Deployment with 3 replicas, PDB with minAvailable=2
- **Test**:
  - Attempt to evict 1 pod - should succeed (2 remain)
  - Attempt to evict 2 pods - should fail (only 1 would remain)
- **Implementation**: ✅ `test_pdb_disruption_prevention` - Passing

#### 4.2 Available Pod Calculation ✅
- **Description**: Test minAvailable/maxUnavailable logic
- **Setup**: 5 pods in deployment
- **Test Cases**:
  - PDB with minAvailable=3 - verify 2 disruptions allowed
  - PDB with maxUnavailable=2 - verify 2 disruptions allowed
  - PDB with minAvailable=5 - verify 0 disruptions allowed
- **Implementation**: ✅ `test_pdb_blocks_excessive_evictions` - Passing

#### 4.3 Percentage-Based Budgets ✅
- **Description**: Test "20%" style specifications
- **Setup**: Deployment with 10 pods
- **Test**:
  - PDB with minAvailable="80%" - verify 2 disruptions allowed
  - PDB with maxUnavailable="30%" - verify 3 disruptions allowed
  - Scale deployment to 20 pods - verify calculations update
- **Implementation**: ✅ `test_pdb_percentage_based_values` - Passing

#### 4.4 Selector Matching ✅
- **Description**: Test label selector matching pods
- **Setup**: 10 pods with label app=web, 5 with app=api
- **Test**:
  - PDB with selector app=web - verify only affects 10 web pods
  - Change pod labels - verify PDB updates affected pods
- **Implementation**: ✅ `test_pdb_selector_matching` + `test_pdb_namespace_isolation` + `test_pdb_list_by_namespace` - Passing

#### 4.5 Status Updates ✅
- **Description**: Test PDB status field accuracy
- **Setup**: Deployment with 5 pods, PDB with minAvailable=3
- **Test**:
  - Verify status.currentHealthy=5
  - Verify status.desiredHealthy=3
  - Verify status.disruptionsAllowed=2
  - Verify status.expectedPods=5
- **Implementation**: ✅ `test_pdb_with_conditions` - Passing

**Summary**: All 5 PDB test scenarios implemented with 7 passing tests covering disruption prevention, eviction blocking, percentage budgets, selector matching, namespace isolation, and status updates.

---

## 5. Init Containers Integration Tests (Medium Priority) ✅ COMPLETE

**Current Status**: ✅ 7 tests passing
**Location**: `crates/kubelet/tests/init_containers_test.rs`
**Estimated Effort**: Low
**Completed**: March 10, 2026

### Test Cases

#### 5.1 Pod Structure with Init Containers ✅
- **Description**: Verify pod spec with init containers
- **Implementation**: ✅ `test_pod_with_init_containers_structure` - Passing
- **Coverage**: Pod creation, init containers array, app containers array

#### 5.2 Status Sequence ✅
- **Description**: Verify init container status transitions
- **Implementation**: ✅ `test_init_container_status_sequence` - Passing
- **Coverage**: Pending phase, init containers running, ready flags

#### 5.3 Completion Flow ✅
- **Description**: Test init containers completing before app starts
- **Implementation**: ✅ `test_init_containers_completed_app_starting` - Passing
- **Coverage**: All init containers Terminated with exit code 0, app container Running

#### 5.4 Failure Handling ✅
- **Description**: Test pod fails if init container fails
- **Implementation**: ✅ `test_init_container_failure_blocks_app` - Passing
- **Coverage**: Failed phase, init container exit code 1, app container never starts

#### 5.5 Restart Count ✅
- **Description**: Verify restart count tracking for failing init containers
- **Implementation**: ✅ `test_init_container_restart_count` - Passing
- **Coverage**: CrashLoopBackOff, restart count increments

#### 5.6 Sequential Execution ✅
- **Description**: Verify multiple init containers in sequence
- **Implementation**: ✅ `test_multiple_init_containers_sequential_execution` - Passing
- **Coverage**: 5 init containers ordered execution, 2 app containers

#### 5.7 Serialization ✅
- **Description**: Verify JSON serialization/deserialization
- **Implementation**: ✅ `test_pod_serialization_with_init_containers` - Passing
- **Coverage**: Round-trip JSON conversion, camelCase field names

**Summary**: All 7 init container test scenarios implemented and passing. Tests cover structure validation, status transitions, completion flows, failure handling, restart tracking, sequential execution, and serialization.

---

## 6. Admission Webhook End-to-End Tests (Medium Priority)

**Current Status**: 21 unit tests, no real webhook tests
**Location**: `crates/api-server/tests/webhook_e2e_test.rs` (to be created)
**Estimated Effort**: Medium

### Test Cases

#### 6.1 Real Webhook Server
- **Description**: Test with actual HTTP webhook endpoint
- **Setup**: Start mock webhook server on localhost:8080
- **Test**:
  - Register MutatingWebhookConfiguration pointing to localhost:8080
  - Create pod
  - Verify API server sends AdmissionReview to webhook
  - Webhook returns patch
  - Verify pod created with mutation applied

#### 6.2 Mutation Application
- **Description**: Test JSON patches are applied correctly
- **Setup**: Webhook that adds label `mutated=true`
- **Test**:
  - Create pod without label
  - Verify pod created with label `mutated=true`
  - Verify patch operation in logs

#### 6.3 Validation Rejection
- **Description**: Test resource creation blocked by validator
- **Setup**: Webhook that rejects pods with name containing "bad"
- **Test**:
  - Create pod named "good-pod" - succeeds
  - Create pod named "bad-pod" - fails with 403 Forbidden
  - Verify error message from webhook displayed

#### 6.4 Failure Policy
- **Description**: Test Fail vs Ignore policy behavior
- **Setup**: Webhook server that is unreachable
- **Test Cases**:
  - failurePolicy=Fail - pod creation fails with error
  - failurePolicy=Ignore - pod creation succeeds, warning logged

#### 6.5 Timeout Handling
- **Description**: Test webhook timeout scenarios
- **Setup**: Webhook that sleeps for 60 seconds
- **Test**:
  - Set webhook timeout to 5 seconds
  - Create pod
  - Verify request times out after 5 seconds
  - Verify failurePolicy determines outcome

#### 6.6 Multiple Webhooks
- **Description**: Test chain of mutations/validations
- **Setup**: 2 mutating webhooks, 2 validating webhooks
- **Test**:
  - Verify both mutations applied in order
  - Verify both validations run after mutations
  - One validator rejects - verify pod not created

---

## 7. HA Cluster Tests (Medium Priority)

**Current Status**: No automated HA tests
**Location**: `tests/ha_cluster_test.rs` (to be created)
**Estimated Effort**: High

### Test Cases

#### 7.1 API Server Failover
- **Description**: Kill one API server, verify HAProxy routes to others
- **Setup**: 3 API servers behind HAProxy
- **Test**:
  - All 3 API servers running
  - Kill API server 1
  - Verify kubectl requests still succeed (routed to 2 or 3)
  - Verify HAProxy marks server 1 as down

#### 7.2 etcd Cluster Failover
- **Description**: Kill one etcd node, verify cluster continues
- **Setup**: 3-node etcd cluster
- **Test**:
  - Write data to etcd
  - Kill etcd node 1
  - Verify writes still succeed (quorum maintained)
  - Verify reads still succeed
  - Restart node 1 - verify it rejoins cluster

#### 7.3 Controller Failover
- **Description**: Kill leader controller, verify follower takes over
- **Setup**: 2 controller-manager instances with leader election
- **Test**:
  - Verify instance 1 is leader
  - Create deployment
  - Verify instance 1 reconciles it (creates pods)
  - Kill instance 1
  - Verify instance 2 becomes leader within 15 seconds
  - Verify instance 2 continues reconciliation

#### 7.4 Split-Brain Scenarios
- **Description**: Test network partitions
- **Setup**: 3 API servers, 3 etcd nodes
- **Test**:
  - Partition network (isolate 1 API server + 1 etcd node)
  - Verify majority side continues operating
  - Verify minority side cannot write (no quorum)
  - Heal partition - verify cluster reconciles

#### 7.5 Full Cluster Restart
- **Description**: Test graceful shutdown and startup
- **Test**:
  - Shut down all components in order (kubelet → controllers → scheduler → API servers → etcd)
  - Wait 30 seconds
  - Start all components in reverse order
  - Verify cluster recovers
  - Verify existing resources still present

---

## 8. Resource Lifecycle Tests (Low Priority - Good Coverage Already)

**Current Status**: 371 status subresource + 324 garbage collector + 402 TTL tests
**Location**: Expand existing test files
**Estimated Effort**: Low

### Test Cases

#### 8.1 Garbage Collection Chain
- **Description**: Test deep dependency trees
- **Setup**: Resource hierarchy: A → B → C → D (owner references)
- **Test**:
  - Delete A with cascade
  - Verify B, C, D all deleted in correct order
  - Test foreground vs background deletion

#### 8.2 Finalizer Timeout
- **Description**: Test stuck finalizers
- **Setup**: Resource with finalizer that never removes itself
- **Test**:
  - Delete resource
  - Verify deletion timestamp set
  - Verify resource not deleted (finalizer blocking)
  - Manually remove finalizer
  - Verify resource deleted

#### 8.3 TTL Edge Cases
- **Description**: Test TTL=0, negative TTL
- **Test Cases**:
  - Job with ttlSecondsAfterFinished=0 - delete immediately
  - Job with ttlSecondsAfterFinished=-1 - should ignore (invalid)
  - Job with no TTL - never auto-deleted

#### 8.4 Status Subresource Race
- **Description**: Test concurrent status updates
- **Setup**: 2 controllers updating same resource status
- **Test**:
  - Both update simultaneously
  - Verify optimistic concurrency (one succeeds, one gets conflict)
  - Verify loser retries and succeeds

---

## 9. DNS Integration Tests (Low Priority - 15 Tests Already)

**Current Status**: 15 passing integration tests
**Location**: `crates/dns-server/tests/dns_integration_test.rs`
**Estimated Effort**: Low

### Additional Test Cases

#### 9.1 DNS Propagation Delays
- **Description**: Test eventual consistency
- **Setup**: Create service, immediately query DNS
- **Test**:
  - Query may initially fail (propagation delay)
  - Retry with exponential backoff
  - Verify DNS resolves within 30 seconds

#### 9.2 Large Result Sets
- **Description**: Test headless services with 100+ pods
- **Setup**: StatefulSet with 100 pods, headless service
- **Test**:
  - Query DNS for service
  - Verify all 100 pod IPs returned
  - Verify DNS response not truncated
  - Verify query performance acceptable

#### 9.3 NXDOMAIN Handling
- **Description**: Test non-existent service queries
- **Test**:
  - Query for service that doesn't exist
  - Verify NXDOMAIN response
  - Verify no errors logged

#### 9.4 SRV Record Priority
- **Description**: Test multiple SRV records
- **Setup**: Service with 3 ports
- **Test**:
  - Query SRV for each port
  - Verify correct port and protocol in SRV response
  - Verify priority and weight fields

---

## 10. LoadBalancer Integration Tests (Low Priority - 16 Tests Already)

**Current Status**: 16 passing unit tests
**Location**: Expand `crates/controller-manager/tests/` or `crates/cloud-providers/tests/`
**Estimated Effort**: Low

### Additional Test Cases

#### 10.1 AWS NLB Lifecycle
- **Description**: Full create → update → delete flow
- **Setup**: Mock AWS SDK
- **Test**:
  - Create LoadBalancer service
  - Verify NLB created in AWS
  - Update service ports
  - Verify NLB updated
  - Delete service
  - Verify NLB deleted

#### 10.2 Multi-Port Services
- **Description**: Test services with multiple ports
- **Setup**: LoadBalancer service with 3 ports (80, 443, 8080)
- **Test**:
  - Verify NLB has 3 target groups
  - Verify each target group has correct port mapping
  - Verify all listeners created

#### 10.3 Health Check Integration
- **Description**: Test backend health checks
- **Setup**: LoadBalancer with health check annotations
- **Test**:
  - Verify health check configured on target group
  - Verify correct protocol, port, path
  - Verify interval and timeout settings

#### 10.4 Cross-AZ Load Balancing
- **Description**: Test multi-AZ deployments
- **Setup**: Nodes in 3 availability zones
- **Test**:
  - Verify NLB configured for all 3 AZs
  - Verify targets registered in all AZs
  - Verify cross-zone load balancing enabled

---

## Test Infrastructure Needs

### Mock Components

1. **Mock CNI Plugin** (`tests/fixtures/mock-cni-plugin.sh`)
   - Shell script that accepts CNI commands
   - Returns valid CNI JSON results
   - Writes to temp file for verification

2. **Mock Webhook Server** (`tests/fixtures/mock-webhook-server`)
   - HTTP server accepting AdmissionReview
   - Configurable responses (allow, deny, mutate)
   - Can be started/stopped in tests

3. **etcd Test Helper** (`tests/common/etcd_helper.rs`) - **ALREADY EXISTS**
   - Start temporary etcd instance
   - Clean up after tests
   - Reusable across test files

4. **Mock Metrics Server**
   - Returns fake CPU/memory metrics
   - For HPA/VPA testing

### Test Fixtures

1. **Sample YAML Resources**
   - `tests/fixtures/hpa-example.yaml`
   - `tests/fixtures/vpa-example.yaml`
   - `tests/fixtures/pdb-example.yaml`
   - `tests/fixtures/init-container-pod.yaml`

2. **CNI Configurations**
   - `tests/fixtures/bridge.conf`
   - `tests/fixtures/chained.conflist`
   - `tests/fixtures/multi-network.conflist`

---

## Implementation Priority

### Phase 1: Quick Wins (1-2 weeks) ✅ **COMPLETE**
- [x] PDB Controller Tests (Low effort, no tests currently) ✅ **DONE**
- [x] Init Containers E2E Tests (Low effort, common use case) ✅ **DONE**
- [x] CNI Mock Plugin Tests (Medium effort, high value) ✅ **DONE**

### Phase 2: High Priority (2-4 weeks)
- [ ] Leader Election Integration Tests (Critical for HA)
- [x] CNI Integration Tests (Critical for networking) ✅ **DONE**
- [ ] Admission Webhook E2E Tests (Complex feature needs validation)

### Phase 3: Medium Priority (4-6 weeks)
- [ ] HPA/VPA Controller Integration Tests (New features)
- [ ] HA Cluster Tests (Important for production)

### Phase 4: Expansions (Ongoing)
- [ ] Resource Lifecycle Edge Cases
- [ ] DNS Expansion Tests
- [ ] LoadBalancer Cloud Provider Tests

---

## Test Coverage Goals

| Component | Current Coverage | Target Coverage | Gap |
|-----------|------------------|-----------------|-----|
| CNI Framework | ✅ 16 unit + 9 integration | Unit + Integration | ✅ Complete |
| Leader Election | 1 ignored test | Full integration suite | 5+ tests needed |
| HPA/VPA | 3 unit tests | Unit + Integration | Integration tests needed |
| PDB | ✅ 7 integration tests | Full suite | ✅ Complete |
| Init Containers | ✅ 7 integration tests | Unit + E2E | ✅ Complete |
| Admission Webhooks | 21 unit tests | Unit + E2E | E2E tests needed |
| HA Cluster | 0 tests | Full integration | All tests needed |

**Overall Goal**: 95%+ code coverage with mix of unit, integration, and E2E tests

---

## Running Tests

### Unit Tests
```bash
cargo test --lib
```

### Integration Tests
```bash
cargo test --test '*'
```

### Specific Test Suite
```bash
cargo test --test cni_integration_test
cargo test --test leader_election_test
cargo test --test hpa_controller_test
```

### With etcd (for leader election tests)
```bash
# Start etcd first
docker run -d -p 2379:2379 quay.io/coreos/etcd:v3.5.17 \
  /usr/local/bin/etcd --listen-client-urls http://0.0.0.0:2379 \
  --advertise-client-urls http://localhost:2379

# Run tests
cargo test --test leader_election_test -- --include-ignored
```

---

## Contributing

When implementing tests from this list:

1. Update the status column to ✅ Complete
2. Add link to PR in notes column
3. Document any new test fixtures created
4. Update test coverage metrics
5. Add any new dependencies to relevant Cargo.toml

---

## Notes

- Some tests require external dependencies (etcd, AWS credentials)
- Consider using Docker for test isolation
- Mock external services where possible
- Focus on testing behavior, not implementation details
- Keep tests fast - use mocks for slow operations

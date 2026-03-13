#!/bin/bash
set -e

# Conformance test runner for Rusternetes
# This script handles the full lifecycle of running Kubernetes conformance tests

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Rusternetes Conformance Test Runner ==="
echo ""

# Setup kubeconfig
export KUBECONFIG=~/.kube/rusternetes-config

# Step 1: Kill any running sonobuoy processes
echo "[1/5] Cleaning up old sonobuoy processes..."
pkill -f "sonobuoy run" || true
sleep 2

# Step 2: Delete sonobuoy resources from cluster (ignore errors)
echo "[2/5] Cleaning up sonobuoy resources..."
sonobuoy delete --wait 2>/dev/null || true
sleep 2

# Step 3: Ensure CoreDNS is running
echo "[3/5] Checking CoreDNS status..."
COREDNS_STATUS=$(curl -sk https://localhost:6443/api/v1/namespaces/kube-system/pods/coredns 2>/dev/null | grep -o '"phase":"[^"]*"' | cut -d'"' -f4 || echo "NotFound")

if [ "$COREDNS_STATUS" != "Running" ]; then
    echo "CoreDNS not running (status: $COREDNS_STATUS), recreating..."
    # Delete if it exists
    curl -sk -X DELETE https://localhost:6443/api/v1/namespaces/kube-system/pods/coredns >/dev/null 2>&1 || true
    sleep 2
    # Recreate via bootstrap script (includes ServiceAccount/token generation)
    ./scripts/bootstrap-cluster.sh
else
    echo "CoreDNS is already running"
fi

# Step 4: Run conformance tests
echo "[4/5] Starting conformance tests (this will take several minutes)..."
echo "Running: sonobuoy run --mode=quick --wait"
echo ""

# Run sonobuoy and capture output
if sonobuoy run --mode=quick --wait 2>&1 | tee /tmp/sonobuoy-latest.log; then
    TEST_RESULT="PASSED"
else
    TEST_RESULT="FAILED"
fi

# Step 5: Retrieve and display results
echo ""
echo "[5/5] Retrieving test results..."
echo ""

# Get the results
RESULTS_FILE=$(sonobuoy retrieve 2>/dev/null || echo "")
if [ -n "$RESULTS_FILE" ]; then
    echo "Results saved to: $RESULTS_FILE"
    echo ""
    echo "=== Test Summary ==="
    sonobuoy results "$RESULTS_FILE" 2>/dev/null || echo "Could not parse results"
    echo ""
    echo "=== Detailed Results ==="
    sonobuoy results "$RESULTS_FILE" --mode=detailed 2>/dev/null || echo "Could not get detailed results"
else
    echo "WARNING: Could not retrieve results file"
fi

echo ""
echo "=== Conformance Test Complete ==="
echo "Overall Status: $TEST_RESULT"
echo "Full log saved to: /tmp/sonobuoy-latest.log"
echo ""

if [ "$TEST_RESULT" == "FAILED" ]; then
    exit 1
fi

#!/bin/bash
# Conformance test progress monitor for Rusternetes
# Parses e2e container logs to show real-time pass/fail counts
# since sonobuoy's built-in progress reporting doesn't work with K8s v1.35
#
# Usage: bash scripts/conformance-progress.sh [interval_seconds]

INTERVAL="${1:-10}"

# Find the e2e pod name
find_e2e_pod() {
    curl -sk https://localhost:6443/api/v1/namespaces/sonobuoy/pods 2>/dev/null | \
        python3 -c "
import sys,json
try:
    data=json.load(sys.stdin)
    for p in data.get('items',[]):
        if 'e2e-job' in p['metadata']['name']:
            print(p['metadata']['name'])
            break
except: pass
" 2>/dev/null
}

parse_progress() {
    python3 -c "
import sys
lines = sys.stdin.read().split('\n')
passed = 0
failed = 0
completed = False
last_test = ''
for line in lines:
    if line.startswith('\u2022 [FAILED]'):
        failed += 1
    elif line.startswith('\u2022 ['):
        passed += 1
    elif line.startswith('\u2022'):
        passed += 1
    if line.startswith('[sig-') or line.startswith('[k8s.io'):
        last_test = line[:120]
    if line.startswith('Ran '):
        completed = True

total = 441
done = passed + failed
remaining = max(0, total - done)
pct = f'{passed * 100 / done:.1f}' if done > 0 else '0.0'
print(f'{passed}|{failed}|{remaining}|{total}|{pct}|{1 if completed else 0}|{last_test}')
"
}

echo "=== Rusternetes Conformance Progress Monitor ==="
echo "Polling every ${INTERVAL}s (pass interval as arg to change)"
echo ""

while true; do
    E2E_POD=$(find_e2e_pod)
    if [ -z "$E2E_POD" ]; then
        echo "$(date +%H:%M:%S) | No e2e pod found. Waiting..."
        sleep "$INTERVAL"
        continue
    fi

    # Get logs and parse results
    RESULT=$(curl -sk "https://localhost:6443/api/v1/namespaces/sonobuoy/pods/${E2E_POD}/log?container=e2e" 2>/dev/null | parse_progress)

    if [ -z "$RESULT" ]; then
        echo "$(date +%H:%M:%S) | No logs yet. Waiting..."
        sleep "$INTERVAL"
        continue
    fi

    IFS='|' read -r PASSED FAILED REMAINING TOTAL PASS_RATE IS_COMPLETE LAST_TEST <<< "$RESULT"
    DONE=$((PASSED + FAILED))

    echo "$(date +%H:%M:%S) | Passed: ${PASSED} | Failed: ${FAILED} | Done: ${DONE}/${TOTAL} | Remaining: ${REMAINING} | Pass rate: ${PASS_RATE}%"

    if [ "$IS_COMPLETE" = "1" ]; then
        echo ""
        echo "=== Suite Complete ==="
        echo "Final: Passed=${PASSED} Failed=${FAILED} Total=${DONE}"
        break
    fi

    sleep "$INTERVAL"
done

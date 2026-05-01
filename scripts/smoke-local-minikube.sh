#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/smoke-common.sh"

[ "${SKIP_SMOKE:-0}" = "1" ] && skip "SKIP_SMOKE=1"
need_cmd kubectl
driver="$(runtime_driver)"
runtime_ready "$driver"
profile="${RUSTERNETES_SMOKE_PROFILE:-smoke-minikube}"
trap 'cleanup_profile "$profile"' EXIT

rusternetes_cli cluster create --profile "$profile" --driver "$driver" --mode minikube --force
rusternetes_cli cluster stop --profile "$profile"
rusternetes_cli cluster start --profile "$profile"
smoke_workload "$profile"
echo "PASS: local minikube-compatible smoke test"

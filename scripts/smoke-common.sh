#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

skip() {
  echo "SKIP: $*"
  exit 0
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || skip "$1 is not installed"
}

runtime_driver() {
  if [ -n "${RUSTERNETES_DRIVER:-}" ]; then
    echo "$RUSTERNETES_DRIVER"
  elif command -v docker >/dev/null 2>&1; then
    echo docker
  elif command -v podman >/dev/null 2>&1; then
    echo podman
  else
    skip "neither docker nor podman is installed"
  fi
}

runtime_ready() {
  local driver="$1"
  case "$driver" in
    docker) docker info >/dev/null 2>&1 || skip "docker is installed but not running" ;;
    podman) podman info >/dev/null 2>&1 || skip "podman is installed but not running" ;;
    *) skip "unknown runtime driver $driver" ;;
  esac
}

rusternetes_cli() {
  if [ -n "${RUSTERNETES_BIN:-}" ]; then
    "$RUSTERNETES_BIN" "$@"
  elif [ -x "$repo_root/target/debug/rusternetes" ]; then
    "$repo_root/target/debug/rusternetes" "$@"
  else
    cargo run -q -p rusternetes-cli -- "$@"
  fi
}

cleanup_profile() {
  local profile="$1"
  if [ "${KEEP_CLUSTER:-0}" = "1" ]; then
    echo "KEEP_CLUSTER=1; leaving profile $profile running"
  else
    rusternetes_cli cluster logs --profile "$profile" || true
    rusternetes_cli cluster delete --profile "$profile" || true
  fi
}

smoke_workload() {
  local profile="$1"
  local kubeconfig
  kubeconfig="$(rusternetes_cli cluster kubeconfig --profile "$profile" --path-only)"
  export KUBECONFIG="$kubeconfig"
  kubectl get nodes
  kubectl apply -f examples/local/configmap.yaml
  kubectl apply -f examples/local/nginx-deployment.yaml
  kubectl apply -f examples/local/nginx-service.yaml
  kubectl get deployment rusternetes-nginx
  kubectl get pods -l app=rusternetes-nginx
  kubectl get service rusternetes-nginx
}

#!/usr/bin/env bash
set -euo pipefail

src="$(cd "$(dirname "$0")" && pwd)"
dest="${1:-$(pwd)}"

if [ ! -f "$dest/Cargo.toml" ] || [ ! -d "$dest/crates" ]; then
  echo "usage: $0 /path/to/rusternetes" >&2
  echo "destination must be a Rūsternetes repository root with Cargo.toml and crates/" >&2
  exit 2
fi

copy_path() {
  local rel="$1"
  mkdir -p "$dest/$(dirname "$rel")"
  rm -rf "$dest/$rel"
  cp -R "$src/$rel" "$dest/$rel"
}

copy_path crates/rusternetes-cluster-manager
copy_path crates/rusternetes-cli
copy_path docker/local-node.Dockerfile
copy_path desktop
copy_path docs/local-clusters.md
copy_path docs/desktop-app.md
copy_path docs/production-single-node.md
copy_path docs/addons.md
copy_path docs/compatibility/kind.md
copy_path docs/compatibility/minikube.md
copy_path examples/local/nginx-pod.yaml
copy_path examples/local/nginx-deployment.yaml
copy_path examples/local/nginx-service.yaml
copy_path examples/local/configmap.yaml
copy_path examples/local/namespace.yaml
copy_path examples/local/volume.yaml
copy_path examples/local/ingress.yaml
copy_path scripts/smoke-common.sh
copy_path scripts/smoke-local-kind.sh
copy_path scripts/smoke-local-minikube.sh
copy_path scripts/smoke-prod-single-node.sh
copy_path deploy/production-single-node
copy_path .github/workflows/local-cluster.yml

python3 - "$dest" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])

cargo = root / "Cargo.toml"
text = cargo.read_text()
for member in ["crates/rusternetes-cluster-manager", "crates/rusternetes-cli"]:
    if f'"{member}"' not in text:
        anchor = '"crates/rusternetes",'
        if anchor not in text:
            raise SystemExit(f"could not find {anchor} in Cargo.toml")
        text = text.replace(anchor, anchor + f'\n "{member}",', 1)
cargo.write_text(text)

makefile = root / "Makefile"
make_text = makefile.read_text() if makefile.exists() else ""
marker = "# --- Rūsternetes local cluster platform targets ---"
block = r'''
# --- Rūsternetes local cluster platform targets ---
RUSTERNETES_PROFILE ?= dev
RUSTERNETES_DRIVER ?= docker
RUSTERNETES_MODE ?= kind
RUSTERNETES_PROFILE_TYPE ?= development
RUSTERNETES_BIN ?= cargo run -q -p rusternetes-cli --

.PHONY: local-up local-down local-reset local-status local-kubeconfig local-smoke desktop-dev desktop-build

local-up:
	$(RUSTERNETES_BIN) cluster create --profile $(RUSTERNETES_PROFILE) --driver $(RUSTERNETES_DRIVER) --mode $(RUSTERNETES_MODE) --profile-type $(RUSTERNETES_PROFILE_TYPE)

local-down:
	$(RUSTERNETES_BIN) cluster delete --profile $(RUSTERNETES_PROFILE)

local-reset:
	$(RUSTERNETES_BIN) cluster reset --profile $(RUSTERNETES_PROFILE)

local-status:
	$(RUSTERNETES_BIN) cluster status --profile $(RUSTERNETES_PROFILE)

local-kubeconfig:
	$(RUSTERNETES_BIN) cluster kubeconfig --profile $(RUSTERNETES_PROFILE)

local-smoke:
	bash scripts/smoke-local-kind.sh

desktop-dev:
	@if command -v npm >/dev/null 2>&1; then \
		cd desktop && npm install && npm run tauri dev; \
	else \
		echo "npm is required for desktop-dev" >&2; exit 2; \
	fi

desktop-build:
	@if ! command -v npm >/dev/null 2>&1; then \
		echo "SKIP: npm is not installed; desktop build dependencies are unavailable"; \
	elif [ "$${RUSTERNETES_SKIP_DESKTOP_NPM:-0}" = "1" ]; then \
		echo "SKIP: RUSTERNETES_SKIP_DESKTOP_NPM=1"; \
	else \
		if command -v timeout >/dev/null 2>&1; then install_cmd="timeout $${NPM_INSTALL_TIMEOUT:-60} npm install --no-audit --no-fund"; else install_cmd="npm install --no-audit --no-fund"; fi; \
		if ! (cd desktop && sh -c "$$install_cmd"); then \
			echo "SKIP: unable to install desktop npm dependencies in this environment"; \
		else \
			cd desktop && npm run tauri build; \
		fi; \
	fi
'''
if marker not in make_text:
    make_text = make_text.rstrip() + "\n\n" + block.lstrip()
    makefile.write_text(make_text)

readme = root / "README.md"
readme_text = readme.read_text() if readme.exists() else "# Rūsternetes\n"
readme_marker = "## Local cluster platform"
readme_block = r'''
## Local cluster platform

Install the CLI:

```bash
cargo install --path crates/rusternetes-cli
```

Create a kind-compatible local profile:

```bash
rusternetes cluster create --profile dev --mode kind
export KUBECONFIG="$(rusternetes cluster kubeconfig --profile dev --path-only)"
kubectl get nodes
kubectl apply -f examples/local/nginx-deployment.yaml
rusternetes cluster status --profile dev
rusternetes cluster delete --profile dev
```

Create a minikube-compatible local profile:

```bash
rusternetes cluster create --profile dev --driver docker --mode minikube
```

Create a production single-node profile:

```bash
rusternetes cluster create --profile prod-single --mode single-node --profile-type production
```

See `docs/local-clusters.md`, `docs/desktop-app.md`, `docs/production-single-node.md`, `docs/addons.md`, and the compatibility matrices in `docs/compatibility/`.
'''
if readme_marker not in readme_text:
    readme_text = readme_text.rstrip() + "\n\n" + readme_block.lstrip()
    readme.write_text(readme_text)
PY

chmod +x "$dest/scripts/smoke-common.sh" \
  "$dest/scripts/smoke-local-kind.sh" \
  "$dest/scripts/smoke-local-minikube.sh" \
  "$dest/scripts/smoke-prod-single-node.sh" \
  "$dest/deploy/production-single-node/backup.sh" \
  "$dest/deploy/production-single-node/restore.sh"

echo "Rūsternetes local cluster platform overlay applied to $dest"

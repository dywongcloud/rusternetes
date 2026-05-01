# Rūsternetes Local Cluster Platform Extension

This archive adds a production-quality first version of local-cluster orchestration for Rūsternetes while keeping the existing Rūsternetes components as the cluster engine.

The new shared crate is `crates/rusternetes-cluster-manager`. The CLI calls that crate, the desktop app calls the same crate through Tauri commands, and Make/scripts call the CLI. Profiles live under `.rusternetes/profiles/<name>/` with config, kubeconfig, logs, runtime metadata, persistent storage, certificates, add-ons, backup hooks, and state.

## Install the CLI

```bash
cargo install --path crates/rusternetes-cli
```

## Kind-compatible local mode

```bash
rusternetes cluster create --profile dev --driver podman --mode kind
export KUBECONFIG="$(rusternetes cluster kubeconfig --profile dev --path-only)"
kubectl get nodes
kubectl apply -f examples/local/nginx-deployment.yaml
rusternetes cluster status --profile dev
rusternetes cluster delete --profile dev
```

## Minikube-compatible local mode

```bash
rusternetes cluster create --profile dev --driver docker --mode minikube
export KUBECONFIG="$(rusternetes cluster kubeconfig --profile dev --path-only)"
kubectl get nodes
rusternetes cluster addons list --profile dev
rusternetes cluster delete --profile dev
```

## Production single-node mode

```bash
rusternetes cluster create --profile prod-single --mode single-node --profile-type production
export KUBECONFIG="$(rusternetes cluster kubeconfig --profile prod-single --path-only)"
kubectl get nodes
rusternetes cluster status --profile prod-single
```

Production profiles generate TLS and client certificates with OpenSSL, run the node container with a restart policy, persist data under the profile directory, and write backup/restore hooks under `.rusternetes/profiles/<name>/hooks/`.

## Make targets

```bash
make local-up
make local-status
make local-kubeconfig
make local-smoke
make local-reset
make local-down
make desktop-dev
make desktop-build
```

Set `RUSTERNETES_PROFILE`, `RUSTERNETES_DRIVER`, and `RUSTERNETES_MODE` to override defaults.

## Add-ons

```bash
rusternetes cluster addons list --profile dev
rusternetes cluster addons enable registry --profile dev
rusternetes cluster addons enable dashboard --profile dev
rusternetes cluster addons enable ingress --profile dev  # fails clearly in this release
```

The registry add-on starts a real `registry:2` container on a deterministic profile port. Ingress and metrics are exposed through the add-on API but report unsupported/planned until their controllers and bootstrap manifests are bundled.

## Desktop app

The desktop app is a lightweight Tauri shell with a modern vanilla TypeScript UI. It uses the same Rust cluster-manager crate through Tauri commands.

```bash
cd desktop
npm install
npm run tauri dev
npm run tauri build
```

## Documentation

- `docs/local-clusters.md`
- `docs/desktop-app.md`
- `docs/production-single-node.md`
- `docs/addons.md`
- `docs/compatibility/kind.md`
- `docs/compatibility/minikube.md`

## Smoke tests

```bash
bash scripts/smoke-local-kind.sh
bash scripts/smoke-local-minikube.sh
bash scripts/smoke-prod-single-node.sh
```

The scripts run real `rusternetes` and `kubectl` commands when Docker/Podman and kubectl are available. They print `SKIP:` and exit cleanly only for missing optional infrastructure; they do not print success unless the commands actually pass.

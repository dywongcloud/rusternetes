# Desktop app

The desktop app is a lightweight Tauri application. Tauri is preferred because the repository is Rust-first and because Tauri lets the desktop app call the same `rusternetes-cluster-manager` crate used by the CLI.

## Commands

```bash
make desktop-dev
make desktop-build
```

Or directly:

```bash
cd desktop
npm install
npm run tauri dev
npm run tauri build
```

## Features

The UI supports:

- create cluster
- choose mode: kind-compatible, minikube-compatible, production single-node
- choose runtime: Docker or Podman
- start, stop, delete, reset
- status
- kubeconfig path
- copy kubeconfig export command
- logs
- add-ons list/enable/disable
- open web console URL
- run smoke test

The desktop Rust commands call the shared cluster-manager crate. They do not duplicate lifecycle logic.

## Platform support

macOS and Linux are the primary targets. Windows support is prepared architecturally through Tauri and the shared Rust API, but production-grade Windows runtime socket detection and service exposure need validation before it should be marked supported.

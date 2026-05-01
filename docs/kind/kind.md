# kind compatibility

Rūsternetes kind-compatible mode is implemented as a node-container local cluster profile.

## Commands that work today

```bash
rusternetes cluster create --profile dev --driver docker --mode kind
rusternetes cluster status --profile dev
rusternetes cluster kubeconfig --profile dev --path-only
rusternetes cluster load-image nginx:latest --profile dev
rusternetes cluster addons list --profile dev
rusternetes cluster addons enable registry --profile dev
rusternetes cluster reset --profile dev
rusternetes cluster delete --profile dev
```

## Compatibility matrix

| Capability | Status | Notes |
|---|---:|---|
| kubectl compatibility | partially supported | Generated kubeconfig points kubectl at the Rūsternetes API server. Actual behavior depends on existing Rūsternetes API coverage. |
| generated kubeconfig | supported | Profile-scoped kubeconfig is written to `.rusternetes/profiles/<name>/kubeconfig.yaml`. |
| local image loading | partially supported | Verifies image exists in mounted Docker/Podman runtime; no fake containerd import. |
| node lifecycle | supported | create/start/stop/status/delete/reset are implemented for node containers. |
| single-node cluster | supported | One control-plane node container is implemented. |
| multi-node cluster | planned | Names, config, and API support node count; worker bootstrap is reserved for future work. |
| Docker-backed runtime | supported | Requires Docker CLI and socket. |
| Podman-backed runtime | partially supported | Requires Podman CLI and Docker-compatible socket. |
| ingress enablement | not yet supported | Command fails clearly until an ingress controller/bootstrap is bundled. |
| service exposure | planned | Deterministic service port range is reserved. |
| persistent volumes | partially supported | Profile volume directory is mounted into the node container. |
| add-ons architecture | supported | Registry supported; dashboard partial; ingress/metrics report gaps. |
| reset/delete/recreate | supported | Profile lifecycle commands remove and recreate runtime resources. |
| local registry | supported | `registry:2` add-on container. |
| CI-friendly usage | partially supported | Smoke scripts skip only when Docker/Podman/kubectl are absent. |
| dev-machine UX | supported | Make targets and CLI mirror local-cluster workflows. |

## Known gaps

This first release does not claim full kind parity. Multi-node worker bootstrap, `kind load docker-image`-style containerd import, kind config file compatibility, and automatic ingress/service helper parity are the main gaps.

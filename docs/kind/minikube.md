# minikube compatibility

Rūsternetes minikube-compatible mode provides a single local profile with start/stop/delete/status, persistent state, add-ons, and generated kubeconfig.

## Commands that work today

```bash
rusternetes cluster create --profile dev --driver docker --mode minikube
rusternetes cluster stop --profile dev
rusternetes cluster start --profile dev
rusternetes cluster status --profile dev
rusternetes cluster kubeconfig --profile dev --path-only
rusternetes cluster addons list --profile dev
rusternetes cluster addons enable registry --profile dev
rusternetes cluster reset --profile dev
rusternetes cluster delete --profile dev
```

## Compatibility matrix

| Capability | Status | Notes |
|---|---:|---|
| kubectl compatibility | partially supported | Uses generated kubeconfig and existing Rūsternetes API surface. |
| generated kubeconfig | supported | Profile kubeconfig mirrors minikube-style profile selection. |
| local image loading | partially supported | Host Docker/Podman image availability is validated. |
| start/stop/delete/status | supported | Single profile lifecycle is implemented. |
| add-ons interface | supported | Registry works; unsupported add-ons fail clearly. |
| service access helpers | planned | Deterministic service range is reserved; helper commands are not implemented yet. |
| persistent state | supported | SQLite state, volumes, logs, metadata, and add-ons are profile-scoped. |
| Docker driver | supported | Requires Docker CLI/socket. |
| Podman driver | partially supported | Requires Docker-compatible Podman socket. |
| production single-node | supported separately | Use `--mode single-node --profile-type production`. |
| local registry | supported | `registry:2` add-on. |
| dashboard | partially supported | Existing Rūsternetes console integration is used when assets are present. |

## Known gaps

This release does not implement minikube VM drivers, tunnel/service URL helpers, driver-specific mount helpers, or every minikube add-on. The mode keeps those as cluster-manager extension points rather than CLI rewrites.

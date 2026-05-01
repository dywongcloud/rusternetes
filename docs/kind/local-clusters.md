# Local clusters

Rūsternetes local clusters are profile-scoped. A profile is stored at:

```text
.rusternetes/profiles/<name>/
  config.env
  state.env
  kubeconfig.yaml
  logs/
  storage/
  volumes/
  certs/
  metadata/
  addons/
  backups/
  hooks/
```

The cluster manager supports Docker and Podman detection. Docker uses `/var/run/docker.sock`; Podman uses `DOCKER_HOST`, `$XDG_RUNTIME_DIR/podman/podman.sock`, `/run/podman/podman.sock`, or a Docker-compatible socket. The node container mounts the runtime socket so the existing Rūsternetes kubelet can manage containers through its Docker-compatible API.

## Deterministic ports

Ports are computed from the profile name and then adjusted if an existing profile or local listener already uses the deterministic window. A profile gets ports for API server, local registry, metrics, and a future service-exposure range. This enables multiple profiles without collisions while keeping the same profile stable across delete/recreate cycles when the original ports are free.

## Kind-compatible mode

```bash
rusternetes cluster create --profile dev --driver docker --mode kind
export KUBECONFIG="$(rusternetes cluster kubeconfig --profile dev --path-only)"
kubectl get nodes
```

This mode creates a node-container profile named `rusternetes-<profile>-control-plane`. Multi-node topology is represented in the API and container naming model; this first version implements single-node creation and rejects non-kind multi-node requests clearly.

## Minikube-compatible mode

```bash
rusternetes cluster create --profile dev --driver docker --mode minikube
rusternetes cluster stop --profile dev
rusternetes cluster start --profile dev
rusternetes cluster delete --profile dev
```

The minikube-compatible mode uses single-profile semantics: one profile, persistent state, start/stop/delete/reset commands, add-on enablement, service helper extension points, and generated kubeconfig.

## Kubeconfig

```bash
rusternetes cluster kubeconfig --profile dev
export KUBECONFIG="$(rusternetes cluster kubeconfig --profile dev --path-only)"
```

Development profiles use a local HTTP endpoint and a development token because the all-in-one binary defaults to skip-auth. Production profiles generate a CA, API server certificate, admin client certificate, and kubeconfig with certificate paths.

## Image loading

```bash
rusternetes cluster load-image nginx:latest --profile dev
```

Because the current node container mounts the host Docker/Podman socket, image loading verifies that the image exists in that runtime and records it in profile metadata. No fake containerd import is reported. A future containerd-backed node image can implement actual archive import under the same API.

## Reset/delete lifecycle

`delete` removes node/add-on containers, the runtime network, and the profile directory. `reset` reads the existing config, deletes the runtime resources, and recreates the profile with the same mode, driver, image, and node count.

# Podman Tips for Rusternetes Development

This guide provides Podman-specific tips and tricks for developing Rusternetes.

## Why Podman?

Podman is a daemonless container engine that's:
- **Rootless** - More secure, runs without root privileges (but Rusternetes needs rootful for kube-proxy)
- **Docker-compatible** - Uses the same CLI and file formats
- **Kubernetes-native** - Can generate Kubernetes YAML from pods
- **Open source** - Part of the OCI (Open Container Initiative)

## Podman vs Docker

Most Docker commands work with Podman:

| Docker | Podman | Notes |
|--------|--------|-------|
| `docker build` | `podman build` | Identical |
| `docker run` | `podman run` | Identical |
| `docker ps` | `podman ps` | Identical |
| `docker-compose` | `podman-compose` | Slightly different |
| Docker Desktop | Podman Desktop | GUI alternative |

## Podman Machine (macOS/Windows)

Podman runs containers in a Linux VM on macOS/Windows:

```bash
# Initialize the VM
podman machine init

# Start the VM
podman machine start

# Check status
podman machine list

# SSH into the VM
podman machine ssh

# Stop the VM
podman machine stop

# Remove the VM
podman machine rm
```

### Podman Machine Configuration

**IMPORTANT: Rusternetes requires rootful mode for kube-proxy iptables access.**

```bash
# Initialize with rootful mode (REQUIRED for Rusternetes)
podman machine init --rootful --cpus 4 --memory 8192 --disk-size 50

# Verify rootful mode is enabled
podman machine inspect podman-machine-default | grep -i rootful
# Should show: "Rootful": true
```

Without rootful mode, kube-proxy will fail with "Permission denied" errors when trying to configure iptables rules for service routing.

## Podman Compose

Install and use podman-compose:

```bash
# Install
pip3 install podman-compose

# Or via Homebrew (macOS)
brew install podman-compose

# Use it (same as docker-compose)
podman-compose -f compose.yml up -d
podman-compose -f compose.yml logs -f
podman-compose -f compose.yml down
```

### Podman Compose vs Docker Compose

Differences to be aware of:

1. **Socket location**: Podman uses `/run/podman/podman.sock` instead of `/var/run/docker.sock`
2. **Rootless by default**: Containers run as your user
3. **No daemon**: Podman doesn't have a background daemon

## Rootless Containers

Podman's killer feature - run containers without root:

```bash
# Run as your user (default)
podman run -d nginx

# Check ownership
podman inspect container_name | grep User

# Volume permissions work differently
podman run -v ./data:/data:Z nginx  # :Z for SELinux context
```

### File Permissions with Rootless

When mounting volumes:

```bash
# Current user owns files
podman run -v ./data:/data nginx

# Specific user (by UID in container)
podman run --user 1000:1000 -v ./data:/data nginx

# SELinux relabeling (if needed)
podman run -v ./data:/data:z nginx  # :z or :Z
```

## Podman Pods

Podman has native pod support (like Kubernetes):

```bash
# Create a pod
podman pod create --name rusternetes-pod -p 6443:6443

# Run containers in the pod
podman run -d --pod rusternetes-pod --name api-server rusternetes/api-server

# All containers share network namespace
podman run -d --pod rusternetes-pod --name scheduler rusternetes/scheduler

# List pods
podman pod list

# Generate Kubernetes YAML from pod
podman generate kube rusternetes-pod > pod.yaml

# Create pod from Kubernetes YAML
podman play kube pod.yaml
```

## Networking

### Podman Networks

```bash
# List networks
podman network ls

# Create a network
podman network create rusternetes-net

# Inspect network
podman network inspect rusternetes-net

# Remove network
podman network rm rusternetes-net
```

### Port Forwarding

```bash
# Forward host port to container
podman run -p 6443:6443 rusternetes/api-server

# All interfaces
podman run -p 0.0.0.0:6443:6443 rusternetes/api-server

# Localhost only
podman run -p 127.0.0.1:6443:6443 rusternetes/api-server
```

## Volume Management

```bash
# List volumes
podman volume ls

# Create volume
podman volume create etcd-data

# Inspect volume
podman volume inspect etcd-data

# Remove volume
podman volume rm etcd-data

# Remove all unused volumes
podman volume prune
```

## Container Management

### Useful Commands

```bash
# List running containers
podman ps

# List all containers (including stopped)
podman ps -a

# Stop all containers
podman stop $(podman ps -aq)

# Remove all containers
podman rm $(podman ps -aq)

# View container logs
podman logs -f container_name

# Execute command in container
podman exec -it container_name /bin/sh

# Inspect container
podman inspect container_name

# Copy files to/from container
podman cp file.txt container_name:/path/
podman cp container_name:/path/file.txt ./
```

### Resource Limits

```bash
# Limit memory
podman run --memory=512m rusternetes/api-server

# Limit CPU
podman run --cpus=2 rusternetes/api-server

# Set both
podman run --memory=512m --cpus=2 rusternetes/api-server
```

## Image Management

```bash
# List images
podman images

# Remove image
podman rmi image_name

# Remove dangling images
podman image prune

# Remove all images
podman rmi $(podman images -aq)

# Build image
podman build -t rusternetes/api-server:latest .

# Tag image
podman tag rusternetes/api-server:latest rusternetes/api-server:v1.0

# Save image to file
podman save -o api-server.tar rusternetes/api-server

# Load image from file
podman load -i api-server.tar
```

## Debugging Containers

### View Container Processes

```bash
# Top processes in container
podman top container_name

# Stats (like top for containers)
podman stats

# Health check
podman healthcheck run container_name
```

### Container Logs

```bash
# Follow logs
podman logs -f container_name

# Last 100 lines
podman logs --tail 100 container_name

# Since timestamp
podman logs --since 2024-03-09T10:00:00 container_name

# Show timestamps
podman logs -t container_name
```

### Enter Container

```bash
# Interactive shell
podman exec -it container_name /bin/sh

# As root (even in rootless container)
podman exec -it --user root container_name /bin/sh

# Run single command
podman exec container_name ps aux
```

## Systemd Integration

Run containers as systemd services:

```bash
# Generate systemd service file
podman generate systemd --new --name api-server > ~/.config/systemd/user/api-server.service

# Reload systemd
systemctl --user daemon-reload

# Enable service
systemctl --user enable api-server

# Start service
systemctl --user start api-server

# Check status
systemctl --user status api-server
```

## Docker Compatibility Alias

Make Podman fully transparent:

```bash
# Add to ~/.bashrc or ~/.zshrc
alias docker=podman
alias docker-compose='podman-compose -f compose.yml'

# Or symlink (requires sudo)
sudo ln -s $(which podman) /usr/local/bin/docker
```

## Performance Tips

1. **Use volume mounts for development**:
   ```bash
   podman run -v ./crates:/app/crates:Z rusternetes/api-server
   ```

2. **Cache layers effectively**:
   - Put dependency installation before code copy in Dockerfile
   - Use multi-stage builds

3. **Allocate enough resources to Podman machine**:
   ```bash
   podman machine init --cpus 4 --memory 8192
   ```

4. **Use local registry for faster image transfers**:
   ```bash
   podman run -d -p 5000:5000 registry:2
   podman tag rusternetes/api-server localhost:5000/api-server
   podman push localhost:5000/api-server
   ```

## Troubleshooting

### "Cannot connect to Podman socket"

```bash
# macOS: Start Podman machine
podman machine start

# Linux: Start Podman service
systemctl --user start podman.socket
```

### "Port already in use"

```bash
# Find what's using the port
podman ps --filter "publish=6443"

# Or use system tools
lsof -i :6443
```

### "Permission denied" on volumes

```bash
# Use :Z flag for SELinux
podman run -v ./data:/data:Z nginx

# Or run as your user
podman run --user $(id -u):$(id -g) -v ./data:/data nginx
```

### "No space left on device"

```bash
# Clean up unused resources
podman system prune -a

# Check Podman machine disk usage (macOS)
podman machine ssh df -h
```

### "Container fails to start"

```bash
# Check logs
podman logs container_name

# Inspect container
podman inspect container_name

# Check events
podman events --since 5m
```

## Podman Desktop (GUI)

Alternative to Docker Desktop:

```bash
# macOS
brew install podman-desktop

# Or download from https://podman-desktop.io/
```

Features:
- Graphical container management
- Image building
- Pod management
- Kubernetes YAML generation
- Extension support

## Podman with Kubernetes

Generate Kubernetes manifests:

```bash
# From running container
podman generate kube container_name > deployment.yaml

# From pod
podman generate kube pod_name > pod.yaml

# Deploy to Kubernetes
kubectl apply -f pod.yaml
```

## Advanced: Quadlet (systemd)

Modern way to run containers with systemd:

```bash
# Create ~/.config/containers/systemd/api-server.container
cat > ~/.config/containers/systemd/api-server.container <<EOF
[Container]
Image=rusternetes/api-server:latest
PublishPort=6443:6443
Volume=etcd-data:/data

[Service]
Restart=always

[Install]
WantedBy=default.target
EOF

# Reload and start
systemctl --user daemon-reload
systemctl --user start api-server
```

## Resources

- [Podman Documentation](https://docs.podman.io/)
- [Podman Desktop](https://podman-desktop.io/)
- [Podman GitHub](https://github.com/containers/podman)
- [Podman Tutorial](https://github.com/containers/podman/blob/main/docs/tutorials/README.md)

## Quick Reference

| Task | Command |
|------|---------|
| Start VM | `podman machine start` |
| Run container | `podman run -d image` |
| List containers | `podman ps` |
| View logs | `podman logs -f name` |
| Execute in container | `podman exec -it name sh` |
| Stop container | `podman stop name` |
| Remove container | `podman rm name` |
| List images | `podman images` |
| Remove image | `podman rmi image` |
| Clean up | `podman system prune -a` |
| Compose up | `podman-compose -f compose.yml up -d` |
| Compose down | `podman-compose -f compose.yml down` |

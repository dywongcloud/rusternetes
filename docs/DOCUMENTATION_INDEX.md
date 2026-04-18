# Rusternetes Documentation Index

This index organizes all Rusternetes documentation for easy navigation.

## Quick Start (Start Here!)

For new users, we recommend this path:

1. **[QUICKSTART.md](QUICKSTART.md)** - Get a cluster running in minutes
2. **[GETTING_STARTED.md](GETTING_STARTED.md)** - Traditional development setup
3. **[DEVELOPMENT.md](DEVELOPMENT.md)** - Complete development guide

## Core Documentation

### Architecture & Design
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System architecture and component design
- **[CONFORMANCE_PLAN.md](planning/CONFORMANCE_PLAN.md)** - Kubernetes conformance tracking
- **[CONFORMANCE_FAILURES.md](CONFORMANCE_FAILURES.md)** - Active conformance fix tracker

### Deployment
- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Production deployment guide
- **[HIGH_AVAILABILITY.md](HIGH_AVAILABILITY.md)** - HA setup with etcd clustering and leader election
- **[PODMAN_TIPS.md](PODMAN_TIPS.md)** - Podman-specific troubleshooting

## Feature Documentation

### API Features
- **[ADVANCED_API_FEATURES.md](ADVANCED_API_FEATURES.md)** - PATCH, Field Selectors, Server-Side Apply
- **[CRD_IMPLEMENTATION.md](CRD_IMPLEMENTATION.md)** - Custom Resource Definitions (CRUD, watch, status/scale subresources, schema validation)

### Networking
- **[LOADBALANCER.md](LOADBALANCER.md)** - LoadBalancer services and cloud providers
- **[METALLB_INTEGRATION.md](METALLB_INTEGRATION.md)** - MetalLB for local LoadBalancer services
- **[CNI_INTEGRATION.md](CNI_INTEGRATION.md)** - Container Network Interface framework
- **[CNI_IMPLEMENTATION_SUMMARY.md](CNI_IMPLEMENTATION_SUMMARY.md)** - CNI implementation details

### Storage
- **[STORAGE_BACKENDS.md](storage/STORAGE_BACKENDS.md)** - Storage backend options (etcd, SQLite/Rhino, memory)
- **[DYNAMIC_PROVISIONING.md](DYNAMIC_PROVISIONING.md)** - Dynamic volume provisioning
- **[VOLUME_SNAPSHOTS.md](VOLUME_SNAPSHOTS.md)** - Volume snapshot feature
- **[VOLUME_EXPANSION.md](VOLUME_EXPANSION.md)** - Volume expansion feature

### Security
- **[AUTHENTICATION.md](AUTHENTICATION.md)** - Authentication, authorization, and cluster security guide
- **[SECURITY.md](SECURITY.md)** - Security features overview (admission, encryption, audit)
- **[WEBHOOK_INTEGRATION.md](WEBHOOK_INTEGRATION.md)** - Admission webhooks integration
- **[TLS_GUIDE.md](TLS_GUIDE.md)** - TLS configuration
- **[Service Account Tokens](security/service-account-tokens.md)** - SA token implementation

### Testing
- **[TESTING.md](testing/TESTING.md)** - Testing procedures
- **[TEST_STATUS.md](testing/TEST_STATUS.md)** - Test status tracking

## Development Guides

### Setup & Configuration
- **[SETUP_NOTES.md](SETUP_NOTES.md)** - Setup and configuration notes
- **[DEV_SETUP_METALLB.md](DEV_SETUP_METALLB.md)** - MetalLB setup for development
- **[LOCAL_DEVELOPMENT.md](LOCAL_DEVELOPMENT.md)** - Local development environment
- **[KUBELET_CONFIGURATION.md](KUBELET_CONFIGURATION.md)** - Kubelet configuration options
- **[DEV_PROCESSES.md](DEV_PROCESSES.md)** - Development processes (redeploy, conformance)

### Contributing
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contribution guidelines

## Implementation Details

- **[WEBSOCKET_EXEC_IMPLEMENTATION.md](WEBSOCKET_EXEC_IMPLEMENTATION.md)** - WebSocket exec implementation
- **[BOOTSTRAP.md](BOOTSTRAP.md)** - Cluster bootstrap process
- **[FIXES_SUMMARY.md](FIXES_SUMMARY.md)** - Summary of fixes (historical)

## Quick Reference

| What do you want to do? | Read this first |
|-------------------------|----------------|
| Get started quickly | [QUICKSTART.md](QUICKSTART.md) |
| Understand the architecture | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Deploy to production | [DEPLOYMENT.md](DEPLOYMENT.md) + [HIGH_AVAILABILITY.md](HIGH_AVAILABILITY.md) |
| Set up development environment | [DEVELOPMENT.md](DEVELOPMENT.md) |
| Use LoadBalancer services | [LOADBALANCER.md](LOADBALANCER.md) + [METALLB_INTEGRATION.md](METALLB_INTEGRATION.md) |
| Implement custom resources | [CRD_IMPLEMENTATION.md](CRD_IMPLEMENTATION.md) |
| Configure networking | [CNI_INTEGRATION.md](CNI_INTEGRATION.md) |
| Secure the cluster | [SECURITY.md](SECURITY.md) + [WEBHOOK_INTEGRATION.md](WEBHOOK_INTEGRATION.md) |
| Track conformance progress | [CONFORMANCE_FAILURES.md](CONFORMANCE_FAILURES.md) |
| Write tests | [TESTING.md](testing/TESTING.md) |

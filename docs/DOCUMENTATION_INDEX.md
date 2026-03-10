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
- **[STATUS.md](STATUS.md)** - Current implementation status and feature matrix
- **[CONFORMANCE_PLAN.md](planning/CONFORMANCE_PLAN.md)** - Kubernetes conformance tracking

### Deployment
- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Production deployment guide
- **[HIGH_AVAILABILITY.md](HIGH_AVAILABILITY.md)** - HA setup with etcd clustering and leader election
- **[PODMAN_TIPS.md](PODMAN_TIPS.md)** - Podman-specific troubleshooting

## Feature Documentation

### API Features
- **[API_FEATURES_COMPLETE.md](API_FEATURES_COMPLETE.md)** - Complete API features overview
- **[ADVANCED_API_FEATURES.md](ADVANCED_API_FEATURES.md)** - PATCH, Field Selectors, Server-Side Apply
- **[PATCH_IMPLEMENTATION.md](PATCH_IMPLEMENTATION.md)** - Detailed PATCH operations guide
- **[CRD_IMPLEMENTATION.md](CRD_IMPLEMENTATION.md)** - Custom Resource Definitions

### Networking
- **[DNS.md](DNS.md)** - DNS server and service discovery
- **[LOADBALANCER.md](LOADBALANCER.md)** - LoadBalancer services and cloud providers
- **[METALLB_INTEGRATION.md](METALLB_INTEGRATION.md)** - MetalLB for local LoadBalancer services
- **[CNI_INTEGRATION.md](CNI_INTEGRATION.md)** - Container Network Interface framework
- **[CNI_IMPLEMENTATION_SUMMARY.md](CNI_IMPLEMENTATION_SUMMARY.md)** - CNI implementation details

### Storage
- **[DYNAMIC_PROVISIONING.md](DYNAMIC_PROVISIONING.md)** - Dynamic volume provisioning
- **[VOLUME_SNAPSHOTS.md](VOLUME_SNAPSHOTS.md)** - Volume snapshot feature
- **[VOLUME_EXPANSION.md](VOLUME_EXPANSION.md)** - Volume expansion feature

### Security
- **[SECURITY.md](SECURITY.md)** - Security features overview
- **[WEBHOOK_INTEGRATION.md](WEBHOOK_INTEGRATION.md)** - Admission webhooks integration
- **[WEBHOOK_TESTING.md](WEBHOOK_TESTING.md)** - Webhook testing guide
- **[TLS_GUIDE.md](TLS_GUIDE.md)** - TLS configuration

### Observability
- **[TRACING.md](TRACING.md)** - Distributed tracing with OpenTelemetry

### Testing
- **[TESTING.md](TESTING.md)** - Testing procedures
- **[TESTING_IMPLEMENTATION_GUIDE.md](TESTING_IMPLEMENTATION_GUIDE.md)** - Comprehensive testing guide

## Development Guides

### Setup & Configuration
- **[SETUP_NOTES.md](SETUP_NOTES.md)** - Setup and configuration notes
- **[DEV_SETUP_METALLB.md](DEV_SETUP_METALLB.md)** - MetalLB setup for development
- **[LOCAL_DEVELOPMENT.md](LOCAL_DEVELOPMENT.md)** - Local development environment
- **[KUBELET_CONFIGURATION.md](KUBELET_CONFIGURATION.md)** - Kubelet configuration options

### Contributing
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contribution guidelines

## Implementation Details & Summaries

These documents track specific implementations and improvements:

- **[CONFORMANCE_IMPLEMENTATION_STATUS.md](CONFORMANCE_IMPLEMENTATION_STATUS.md)** - Detailed conformance status
- **[CONFORMANCE_IMPROVEMENTS_2026-03-10.md](CONFORMANCE_IMPROVEMENTS_2026-03-10.md)** - Recent conformance improvements
- **[METALLB_SETUP_SUMMARY.md](METALLB_SETUP_SUMMARY.md)** - MetalLB setup summary
- **[KUBELET_CONFIG_IMPLEMENTATION.md](KUBELET_CONFIG_IMPLEMENTATION.md)** - Kubelet config implementation
- **[WEBSOCKET_EXEC_IMPLEMENTATION.md](WEBSOCKET_EXEC_IMPLEMENTATION.md)** - WebSocket exec implementation
- **[WEBSOCKET_ATTACH_PORTFORWARD_IMPLEMENTATION.md](WEBSOCKET_ATTACH_PORTFORWARD_IMPLEMENTATION.md)** - WebSocket attach/port-forward

## Legacy/Archive Files

The following files contain historical information or summaries that may be outdated:

- **[DEV_COMPARISON.md](DEV_COMPARISON.md)** - Development environment comparison
- **[FIXES_SUMMARY.md](FIXES_SUMMARY.md)** - Summary of fixes (historical)
- **[KUBECTL_FIX_SUMMARY.md](KUBECTL_FIX_SUMMARY.md)** - kubectl fixes (historical)
- **[TEST_IMPROVEMENTS.md](TEST_IMPROVEMENTS.md)** - Test improvements (historical)
- **[VERIFICATION_REPORT.md](VERIFICATION_REPORT.md)** - Verification report (historical)
- **[CONFORMANCE.md](CONFORMANCE.md)** - Old conformance doc (use [planning/CONFORMANCE_PLAN.md](planning/CONFORMANCE_PLAN.md) instead)

## Documentation Maintenance

**Last Updated**: March 12, 2026

**Note**: If you find outdated information or broken links, please update this index and the relevant documentation files.

### Quick Reference

| What do you want to do? | Read this first |
|-------------------------|----------------|
| Get started quickly | [QUICKSTART.md](QUICKSTART.md) |
| Understand the architecture | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Deploy to production | [DEPLOYMENT.md](DEPLOYMENT.md) + [HIGH_AVAILABILITY.md](HIGH_AVAILABILITY.md) |
| Set up development environment | [DEVELOPMENT.md](DEVELOPMENT.md) |
| Use LoadBalancer services | [LOADBALANCER.md](LOADBALANCER.md) + [METALLB_INTEGRATION.md](METALLB_INTEGRATION.md) |
| Implement custom resources | [CRD_IMPLEMENTATION.md](CRD_IMPLEMENTATION.md) |
| Configure networking | [CNI_INTEGRATION.md](CNI_INTEGRATION.md) + [DNS.md](DNS.md) |
| Secure the cluster | [SECURITY.md](SECURITY.md) + [WEBHOOK_INTEGRATION.md](WEBHOOK_INTEGRATION.md) |
| Track conformance progress | [planning/CONFORMANCE_PLAN.md](planning/CONFORMANCE_PLAN.md) |
| Write tests | [TESTING_IMPLEMENTATION_GUIDE.md](TESTING_IMPLEMENTATION_GUIDE.md) |


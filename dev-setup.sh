#!/bin/bash

# Rusternetes Development Environment Setup Script
# This script helps set up a local development environment using Podman

set -e

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BOLD}${GREEN}======================================${NC}"
    echo -e "${BOLD}${GREEN}  Rusternetes Development Setup${NC}"
    echo -e "${BOLD}${GREEN}======================================${NC}"
}

print_step() {
    echo -e "\n${BOLD}${GREEN}==>${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}WARNING:${NC} $1"
}

print_error() {
    echo -e "${RED}ERROR:${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

check_command() {
    if command -v $1 &> /dev/null; then
        print_success "$1 is installed"
        return 0
    else
        print_error "$1 is not installed"
        return 1
    fi
}

print_header

# Check prerequisites
print_step "Checking prerequisites..."

MISSING_DEPS=0

if ! check_command "cargo"; then
    echo "  Install Rust from https://rustup.rs/"
    MISSING_DEPS=1
fi

# Check for Podman or Docker
CONTAINER_RUNTIME=""
if check_command "podman"; then
    CONTAINER_RUNTIME="podman"
elif check_command "docker"; then
    CONTAINER_RUNTIME="docker"
    print_warning "Docker detected. This project is optimized for Podman."
else
    print_error "Neither Podman nor Docker is installed"
    echo "  Install Podman from https://podman.io/getting-started/installation"
    MISSING_DEPS=1
fi

# Check for podman-compose or docker-compose
COMPOSE_CMD=""
if [ "$CONTAINER_RUNTIME" = "podman" ]; then
    if check_command "podman-compose"; then
        COMPOSE_CMD="podman-compose"
    elif check_command "docker-compose"; then
        COMPOSE_CMD="docker-compose"
        print_warning "Using docker-compose with Podman (experimental)"
    else
        print_error "podman-compose is not installed"
        echo "  Install with: pip3 install podman-compose"
        MISSING_DEPS=1
    fi
else
    if check_command "docker-compose"; then
        COMPOSE_CMD="docker-compose"
    else
        print_error "docker-compose is not installed"
        MISSING_DEPS=1
    fi
fi

if [ $MISSING_DEPS -eq 1 ]; then
    print_error "Missing required dependencies. Please install them and try again."
    exit 1
fi

print_success "All prerequisites are installed!"
echo "  Container runtime: $CONTAINER_RUNTIME"
echo "  Compose command: $COMPOSE_CMD"

# Ask user what they want to do
echo ""
echo "What would you like to do?"
echo "  1) Build all container images"
echo "  2) Start the development cluster"
echo "  3) Stop the development cluster"
echo "  4) Clean up (remove all containers and volumes)"
echo "  5) View logs"
echo "  6) Build Rust binaries locally"
echo "  7) Run tests"
echo "  8) Full setup (build + start)"
echo "  9) Exit"
echo ""
read -p "Enter your choice [1-9]: " choice

case $choice in
    1)
        print_step "Building all container images..."
        $COMPOSE_CMD build
        print_success "All images built successfully!"
        ;;
    2)
        print_step "Starting development cluster..."
        $COMPOSE_CMD up -d
        print_success "Cluster started!"
        echo ""
        echo "API Server is available at: http://localhost:6443"
        echo "etcd is available at: http://localhost:2379"
        echo ""
        echo "View logs with: $COMPOSE_CMD logs -f"
        echo "Stop cluster with: $COMPOSE_CMD down"
        ;;
    3)
        print_step "Stopping development cluster..."
        $COMPOSE_CMD down
        print_success "Cluster stopped!"
        ;;
    4)
        print_step "Cleaning up all resources..."
        read -p "This will remove all containers, volumes, and images. Continue? [y/N]: " confirm
        if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
            $COMPOSE_CMD down -v
            $CONTAINER_RUNTIME rmi $(podman images "rusternetes*" -q) 2>/dev/null || true
            print_success "Cleanup complete!"
        else
            echo "Cleanup cancelled."
        fi
        ;;
    5)
        print_step "Viewing logs..."
        $COMPOSE_CMD logs -f
        ;;
    6)
        print_step "Building Rust binaries..."
        cargo build --release
        print_success "Binaries built successfully!"
        echo "Binaries are available in: target/release/"
        ;;
    7)
        print_step "Running tests..."
        cargo test
        print_success "Tests completed!"
        ;;
    8)
        print_step "Running full setup..."
        print_step "Building container images..."
        $COMPOSE_CMD build
        print_success "Images built!"

        print_step "Starting development cluster..."
        $COMPOSE_CMD up -d
        print_success "Cluster started!"

        echo ""
        print_success "Development environment is ready!"
        echo ""
        echo "API Server: http://localhost:6443"
        echo "etcd: http://localhost:2379"
        echo ""
        echo "Next steps:"
        echo "  - View logs: $COMPOSE_CMD logs -f"
        echo "  - Run kubectl: cargo run --bin kubectl -- --server http://localhost:6443 get pods"
        echo "  - Stop cluster: $COMPOSE_CMD down"
        ;;
    9)
        echo "Exiting..."
        exit 0
        ;;
    *)
        print_error "Invalid choice"
        exit 1
        ;;
esac

print_success "Done!"

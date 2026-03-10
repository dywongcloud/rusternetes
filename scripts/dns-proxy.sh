#!/bin/bash
# DNS UDP Proxy for Rusternetes - LOCAL DEVELOPMENT ONLY
#
# ⚠️  WARNING: This is a development tool for testing DNS from your macOS host.
#              In production, DNS is only meant to be accessible from within the cluster.
#              Pods inside the cluster can resolve services normally without this proxy.
#
# This script creates a UDP proxy to make the DNS server accessible from the macOS host.
# It's needed because Podman Machine on macOS doesn't support UDP port forwarding.
#
# Usage:
#   ./scripts/dns-proxy.sh start    # Start the proxy
#   ./scripts/dns-proxy.sh stop     # Stop the proxy
#   ./scripts/dns-proxy.sh status   # Check proxy status

PROXY_CONTAINER="rusternetes-dns-proxy"
DNS_CONTAINER="rusternetes-dns-server"
NETWORK="rusternetes-network"
HOST_PORT="15353"  # Custom DNS port (standard DNS port 53 requires root, 5353 is mDNS)
CONTAINER_PORT="8053"

start_proxy() {
    echo "=========================================="
    echo "  DNS UDP Proxy - LOCAL DEVELOPMENT ONLY"
    echo "=========================================="
    echo ""
    echo "Starting DNS UDP proxy..."
    echo ""
    echo "ℹ️  Note: This proxy is only needed for testing DNS from your macOS host."
    echo "   Pods inside the cluster can already resolve DNS without this proxy."
    echo ""

    # Check if proxy is already running
    if podman ps --format '{{.Names}}' | grep -q "^${PROXY_CONTAINER}$"; then
        echo "✅ DNS proxy is already running"
        return 0
    fi

    # Remove old proxy container if it exists
    podman rm -f ${PROXY_CONTAINER} 2>/dev/null

    # Start socat proxy container
    # This runs socat inside the Podman network where it CAN reach the DNS container
    podman run -d \
        --name ${PROXY_CONTAINER} \
        --network ${NETWORK} \
        -p ${HOST_PORT}:53/udp \
        alpine/socat \
        -d -d \
        UDP4-LISTEN:53,fork,reuseaddr \
        UDP4:${DNS_CONTAINER}:${CONTAINER_PORT}

    if [ $? -eq 0 ]; then
        echo "✅ DNS proxy started successfully"
        echo "   You can now query DNS at: localhost:${HOST_PORT}"
        echo ""
        echo "   Test with: dig @localhost -p ${HOST_PORT} test.default.svc.cluster.local"
    else
        echo "❌ Failed to start DNS proxy"
        return 1
    fi
}

stop_proxy() {
    echo "Stopping DNS UDP proxy..."
    podman stop ${PROXY_CONTAINER} 2>/dev/null
    podman rm ${PROXY_CONTAINER} 2>/dev/null
    echo "✅ DNS proxy stopped"
}

status_proxy() {
    if podman ps --format '{{.Names}}' | grep -q "^${PROXY_CONTAINER}$"; then
        echo "✅ DNS proxy is running"
        podman ps --filter "name=${PROXY_CONTAINER}" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
    else
        echo "❌ DNS proxy is not running"
        echo "   Start it with: $0 start"
        return 1
    fi
}

case "$1" in
    start)
        start_proxy
        ;;
    stop)
        stop_proxy
        ;;
    status)
        status_proxy
        ;;
    restart)
        stop_proxy
        sleep 1
        start_proxy
        ;;
    *)
        echo "Usage: $0 {start|stop|status|restart}"
        echo ""
        echo "Commands:"
        echo "  start    - Start the DNS UDP proxy"
        echo "  stop     - Stop the DNS UDP proxy"
        echo "  status   - Check if proxy is running"
        echo "  restart  - Restart the proxy"
        exit 1
        ;;
esac

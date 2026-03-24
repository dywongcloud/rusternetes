#!/bin/bash
# Script to generate TLS certificates for the API server
# These certificates are persisted and reused across restarts

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CERT_DIR="${PROJECT_ROOT}/.rusternetes/certs"
CERT_FILE="${CERT_DIR}/api-server.crt"
KEY_FILE="${CERT_DIR}/api-server.key"

# Check if certificates already exist
if [ -f "$CERT_FILE" ] && [ -f "$KEY_FILE" ]; then
    echo "Certificates already exist at:"
    echo "  Cert: $CERT_FILE"
    echo "  Key:  $KEY_FILE"
    echo ""
    echo "To regenerate certificates, delete them first:"
    echo "  rm $CERT_FILE $KEY_FILE"
    exit 0
fi

echo "Generating TLS certificates for API server..."

# Create certs directory if it doesn't exist
mkdir -p "$CERT_DIR"

# Use OpenSSL to generate a self-signed certificate
# This matches the behavior of the Rust TLS generation but persists it

# Generate private key
openssl ecparam -name prime256v1 -genkey -noout -out "$KEY_FILE"

# Create certificate configuration
cat > "${CERT_DIR}/cert.conf" <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = rusternetes-api
O = Rusternetes
C = US

[v3_req]
keyUsage = critical, digitalSignature, keyEncipherment, dataEncipherment, keyCertSign
extendedKeyUsage = serverAuth, clientAuth
basicConstraints = critical, CA:TRUE
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = api-server
DNS.3 = rusternetes-api-server
DNS.4 = kubernetes
DNS.5 = kubernetes.default
DNS.6 = kubernetes.default.svc
DNS.7 = kubernetes.default.svc.cluster.local
IP.1 = 127.0.0.1
IP.2 = 10.96.0.1
IP.3 = 172.18.0.2
IP.4 = 172.18.0.3
IP.5 = 172.18.0.4
IP.6 = 172.18.0.5
EOF

# Generate self-signed certificate (valid for 10 years, matching the Rust implementation)
openssl req -new -x509 \
    -key "$KEY_FILE" \
    -out "$CERT_FILE" \
    -days 3650 \
    -config "${CERT_DIR}/cert.conf" \
    -extensions v3_req \
    -set_serial 01

# Clean up config file
rm "${CERT_DIR}/cert.conf"

# Copy certificate to CoreDNS volume location for ca.crt
COREDNS_CA_DIR="${PROJECT_ROOT}/.rusternetes/volumes/coredns/kube-api-access"
mkdir -p "$COREDNS_CA_DIR"
cp "$CERT_FILE" "${COREDNS_CA_DIR}/ca.crt"
echo "Copied certificate to CoreDNS volume: ${COREDNS_CA_DIR}/ca.crt"

# Also create ca.crt in certs directory for consistency
cp "$CERT_FILE" "${CERT_DIR}/ca.crt"

echo ""
echo "Certificates generated successfully:"
echo "  Cert: $CERT_FILE"
echo "  Key:  $KEY_FILE"
echo "  CA:   ${CERT_DIR}/ca.crt"
echo "  CoreDNS CA: ${COREDNS_CA_DIR}/ca.crt"
echo ""
echo "Certificate details:"
openssl x509 -in "$CERT_FILE" -text -noout | grep -E "(Subject:|Issuer:|Not Before|Not After|DNS:|IP:)"

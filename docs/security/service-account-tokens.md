# ServiceAccount Token Signing

## Overview

Rusternetes implements Kubernetes-compliant ServiceAccount token signing using industry-standard **RS256 (RSA + SHA256)** JWT tokens. This ensures secure authentication for pods accessing the API server.

## Why Token Signing Matters

ServiceAccount tokens are automatically mounted into pods and used to authenticate with the Kubernetes API server. Proper token signing:

- **Prevents token forgery**: Only the controller-manager with the private key can create valid tokens
- **Enables token validation**: The API server can verify tokens using the public key
- **Eliminates MITM attacks**: Tokens cannot be modified in transit
- **Follows Kubernetes standards**: Compatible with standard Kubernetes tooling

## Token Format

Rusternetes generates JWT tokens with the following structure:

### JWT Header
```json
{
  "alg": "RS256",
  "typ": "JWT"
}
```

### JWT Claims
```json
{
  "iss": "rusternetes",
  "sub": "system:serviceaccount:<namespace>:<name>",
  "aud": ["rusternetes"],
  "exp": 1709251200,
  "iat": 1677628800,
  "nbf": 1677628800,
  "kubernetes.io": {
    "namespace": "<namespace>",
    "serviceaccount": {
      "name": "<name>",
      "uid": "<uid>"
    }
  }
}
```

**Field Descriptions:**
- `iss` (issuer): Identifies the token issuer (typically your cluster name)
- `sub` (subject): ServiceAccount identity in format `system:serviceaccount:<namespace>:<name>`
- `aud` (audience): Who the token is intended for (API server identifier)
- `exp` (expiration): Unix timestamp when token expires (default: 1 year)
- `iat` (issued at): Unix timestamp when token was created
- `nbf` (not before): Unix timestamp before which token is not valid
- `kubernetes.io`: Kubernetes-specific claims for namespace and serviceaccount metadata

## Quick Start

> **For Development**: Token signing is **optional** for local development. The controller-manager will work without a signing key and will log a warning. Tokens will be unsigned but functional for testing. **For production deployments**, follow the steps below.

### 1. Generate Signing Keys

Run the provided script to generate an RSA key pair:

```bash
./scripts/generate-sa-signing-key.sh
```

This creates:
- **Private Key**: `~/.rusternetes/keys/sa-signing-key.pem` (2048-bit RSA, permissions: 600)
- **Public Key**: `~/.rusternetes/keys/sa-signing-key.pub` (for API server validation)

### 2. Configure Controller Manager

Set the environment variable to point to the private key:

```bash
export SA_SIGNING_KEY_PATH=~/.rusternetes/keys/sa-signing-key.pem
./target/release/controller-manager
```

Or specify in your systemd service file:

```ini
[Service]
Environment="SA_SIGNING_KEY_PATH=/etc/rusternetes/keys/sa-signing-key.pem"
ExecStart=/usr/local/bin/controller-manager
```

### 3. Configure API Server (Future)

> **Note**: API server token validation is not yet implemented. When implemented, you'll need to configure:

```bash
export SA_PUBLIC_KEY_PATH=~/.rusternetes/keys/sa-signing-key.pub
./target/release/api-server
```

## Production Deployment

### Security Best Practices

#### 1. **Key Storage**

**DO:**
- ✅ Store private keys in a secure key management system (AWS KMS, HashiCorp Vault, etc.)
- ✅ Use file permissions 600 (owner read/write only) for key files
- ✅ Rotate keys regularly (recommended: every 90-180 days)
- ✅ Keep private and public keys in separate locations
- ✅ Use different keys for different clusters

**DON'T:**
- ❌ Commit keys to version control (add `*.pem` to `.gitignore`)
- ❌ Share private keys between environments (dev/staging/prod)
- ❌ Store keys in container images
- ❌ Use the same key for multiple clusters
- ❌ Give keys overly permissive file permissions

#### 2. **Key Rotation**

When rotating keys:

1. Generate a new key pair
2. Configure controller-manager with the new private key
3. Keep the old public key available for validation (grace period)
4. After all tokens expire or are refreshed, remove the old public key
5. Document the rotation in your security audit log

#### 3. **Key Backup**

- Store encrypted backups in a secure location
- Test backup restoration procedure
- Document the backup/restore process
- Limit access to backups (principle of least privilege)

### High Availability Setup

For HA deployments with multiple controller-managers:

1. **Shared Key Approach** (Simple):
   - All controller-manager instances use the same private key
   - Distribute key securely via configuration management (Ansible, Puppet, etc.)
   - Ensures consistent token signing across all instances

2. **Key Management Service** (Recommended):
   - Use AWS KMS, Google Cloud KMS, or Azure Key Vault
   - Controller-managers fetch keys at runtime
   - Automatic key rotation support
   - Centralized audit logging

Example with AWS KMS (future enhancement):
```bash
export SA_SIGNING_KEY_KMS_ARN=arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012
./target/release/controller-manager --use-kms
```

### Monitoring and Auditing

Monitor the following:

1. **Key Loading Errors**:
   ```bash
   # Check controller-manager logs
   grep "Failed to load ServiceAccount signing key" /var/log/rusternetes/controller-manager.log
   ```

2. **Unsigned Token Warnings**:
   ```bash
   # Alert when running without signing key
   grep "No signing key available" /var/log/rusternetes/controller-manager.log
   ```

3. **Token Creation Rate**:
   - Monitor the rate of ServiceAccount and Secret creation
   - Unusual spikes may indicate issues or attacks

## Token Expiration

Default token expiration: **1 year** (365 days)

### Configuring Expiration

> **Future Enhancement**: Token expiration will be configurable via:

```bash
export SA_TOKEN_EXPIRATION_SECONDS=31536000  # 1 year
export SA_TOKEN_EXPIRATION_SECONDS=2592000   # 30 days (more secure)
```

### Token Refresh

Kubernetes automatically refreshes ServiceAccount tokens before expiration. Rusternetes will implement this in a future release.

## Troubleshooting

### Controller Manager Won't Start

**Symptom**: Controller manager fails to start or crashes

**Common Causes**:
1. Private key file not found
2. Invalid key format
3. Incorrect file permissions

**Solutions**:
```bash
# Check if key exists
ls -la ~/.rusternetes/keys/sa-signing-key.pem

# Verify key format
openssl rsa -in ~/.rusternetes/keys/sa-signing-key.pem -check

# Fix permissions
chmod 600 ~/.rusternetes/keys/sa-signing-key.pem

# Check controller-manager logs
journalctl -u rusternetes-controller-manager -f
```

### Tokens Not Being Signed

**Symptom**: Warning logs about "No signing key available"

**Cause**: Controller manager couldn't load the signing key

**Solutions**:
1. Verify `SA_SIGNING_KEY_PATH` environment variable is set
2. Check file exists and is readable
3. Verify key format with `openssl rsa -in <key> -check`

### Pods Can't Authenticate

**Symptom**: Pods receive 401 Unauthorized from API server

**Causes**:
1. API server doesn't have the public key (validation not implemented yet)
2. Token expired
3. Token was tampered with

**Future Fix**: Implement token validation in API server using the public key

## Migration from Unsigned Tokens

If you're upgrading from unsigned tokens:

1. Generate signing keys (as above)
2. Configure controller-manager with `SA_SIGNING_KEY_PATH`
3. Restart controller-manager
4. **Gradual Migration**: New tokens will be signed, old tokens still work until recreated
5. Force token rotation (optional):
   ```bash
   # Delete all ServiceAccount token secrets
   kubectl delete secrets --all-namespaces -l "type=kubernetes.io/service-account-token"
   # Controller will recreate them with signatures
   ```

## Advanced Topics

### Custom Token Claims

To customize token claims (future enhancement), modify:
- `crates/controller-manager/src/controllers/serviceaccount.rs`
- Update the `ServiceAccountClaims` struct
- Rebuild controller-manager

### Multiple Signing Keys (Key Rotation)

Future enhancement will support multiple keys for zero-downtime rotation:

```bash
export SA_SIGNING_KEYS=/etc/rusternetes/keys/sa-key-1.pem,/etc/rusternetes/keys/sa-key-2.pem
```

## Security Considerations

### Key Compromise

If you suspect the private key has been compromised:

1. **Immediate Actions**:
   - Generate a new key pair immediately
   - Update controller-manager with new key
   - Revoke all existing ServiceAccount tokens
   - Audit cluster access logs

2. **Recovery**:
   - Force recreation of all ServiceAccount tokens
   - Review and rotate all other cluster credentials
   - Investigate the source of the compromise
   - Update security procedures

3. **Prevention**:
   - Regular key rotation (every 90-180 days)
   - Use hardware security modules (HSM) in production
   - Implement strict access controls
   - Monitor key access and usage

## Reference

- **Kubernetes ServiceAccount Documentation**: https://kubernetes.io/docs/tasks/configure-pod-container/configure-service-account/
- **JWT RFC 7519**: https://tools.ietf.org/html/rfc7519
- **RS256 Algorithm**: https://tools.ietf.org/html/rfc7518#section-3.3

## Files

- **Implementation**: `crates/controller-manager/src/controllers/serviceaccount.rs`
- **Key Generation Script**: `scripts/generate-sa-signing-key.sh`
- **Default Key Location**: `~/.rusternetes/keys/sa-signing-key.pem` (private), `~/.rusternetes/keys/sa-signing-key.pub` (public)

## Support

For issues or questions:
1. Check controller-manager logs for errors
2. Verify key format with OpenSSL
3. Review this documentation
4. Open an issue on GitHub with logs and configuration details

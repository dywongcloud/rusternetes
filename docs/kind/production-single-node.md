# Production single-node Rūsternetes

Production single-node mode targets edge, lab, customer, and air-gapped environments where one durable Kubernetes-compatible control plane is preferable to a developer-only cluster.

```bash
rusternetes cluster create --profile prod-single --mode single-node --profile-type production
export KUBECONFIG="$(rusternetes cluster kubeconfig --profile prod-single --path-only)"
kubectl get nodes
```

## Production defaults

Production profiles enable:

- persistent SQLite state under `.rusternetes/profiles/<profile>/storage/`
- persistent pod volumes under `.rusternetes/profiles/<profile>/volumes/`
- generated CA, API server certificate, and admin client certificate under `certs/`
- kubeconfig using the generated CA and admin certificate
- node container `--restart unless-stopped`
- logs under `logs/`
- backup and restore hooks under `hooks/`
- controlled-upgrade policy metadata under `hooks/upgrade-policy.env`
- deterministic API and registry ports

The profile creation fails clearly if OpenSSL is unavailable because production mode does not fall back to development TLS shortcuts.

## Backup and restore

```bash
.rusternetes/profiles/prod-single/hooks/backup.sh
.rusternetes/profiles/prod-single/hooks/restore.sh .rusternetes/profiles/prod-single/backups/rusternetes-YYYYMMDDHHMMSS.db
rusternetes cluster stop --profile prod-single
rusternetes cluster start --profile prod-single
```

The backup hook copies the SQLite database into `backups/`. The restore hook stages a replacement database; restart the profile container to load it.

## Compose deployment option

A compose template is included at `deploy/production-single-node/compose.yml`. It runs the local-node image directly with persistent bind mounts and a restart policy. Generate certificates with the CLI first, or provide your own CA/server/client material under the same profile `certs/` directory.

```bash
cd deploy/production-single-node
cp .env.example .env
../../target/debug/rusternetes cluster create --profile prod-single --mode single-node --profile-type production --no-wait
docker compose up -d
```

## Systemd option

`deploy/production-single-node/rusternetes-single-node.service` runs the CLI-managed profile through systemd. Install the CLI at `/usr/local/bin/rusternetes`, copy the service, then enable it.

```bash
sudo cp deploy/production-single-node/rusternetes-single-node.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now rusternetes-single-node.service
```

## Upgrade control

The first release uses backup-before-recreate semantics. Before replacing the node image, run the backup hook, update `RUSTERNETES_NODE_IMAGE` or pass `--node-image`, and run:

```bash
rusternetes cluster reset --profile prod-single
```

The cluster-manager API has a stable point for adding pre-flight checks, schema migrations, and rollback snapshots without changing CLI or desktop UX.

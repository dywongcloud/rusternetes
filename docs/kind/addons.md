# Add-ons

The add-on registry is implemented in the shared cluster-manager crate. CLI and desktop app use the same add-on API.

```bash
rusternetes cluster addons list --profile dev
rusternetes cluster addons enable registry --profile dev
rusternetes cluster addons disable registry --profile dev
```

## Registry

Status: supported.

`registry` starts a real `registry:2` container named `rusternetes-<profile>-registry`, persists data under the profile add-on directory, and exposes it on the profile registry port.

## Dashboard / web console

Status: partially supported.

The Rūsternetes API server already supports a web console path when console assets are present. The add-on command validates that console assets exist in the repo and records the desired state. Recreate the profile after building console assets if the running image did not include them.

## Ingress

Status: unsupported in this release.

The command path exists, but enabling ingress fails with an actionable message until an ingress controller image/bootstrap and service routing integration are included. This avoids fake success while preserving the extension point.

## Metrics

Status: planned.

Metrics API wiring is reserved in the registry. It reports planned rather than enabled until the metrics API and aggregation/bootstrap path are implemented.

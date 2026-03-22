# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 51 — exec working, tests progressing)

## BREAKTHROUGH: Exec now works!
WebSocket v5.channel.k8s.io exec with direct Docker execution works.
Tests are progressing past StatefulSet exec operations.

## Round 51 Failures So Far (14 of ~441):

1. StatefulSet scaling — watch closed (transient error handling)
2. Variable Expansion backticks — FIX READY (reject backticks in subpath)
3. Downward API hostIPs — HOST_IP env var not set correctly
4. Secrets env var names — secret data not accessible as env var
5. Aggregator API Server — deployment not becoming available
6. VolumeAttachment lifecycle — delete collection not allowed + status missing
7. Endpoints lifecycle — initial RV not supported in watch
8. DeviceClass creation — Kind missing from response
9. Job failure detection — timeout
10. HostAliases — hosts file not containing aliases
11-14. Various timeouts and watch issues

## Key issues to fix next:
- VolumeAttachment: deletecollection route + status subresource
- Endpoints: watch with empty RV
- HostAliases in /etc/hosts
- HOST_IP downward API env var
- Secret env var injection

## 37+ fixes deployed including exec breakthrough

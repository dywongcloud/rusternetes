# Conformance Issue Tracker

**281 total fixes** | Round 105 in progress | 7 failures so far

## Round 105 Failures
| Test | Error | Root cause | Fix needed |
|------|-------|------------|-----------|
| statefulset.go:786 | SS scaling — watch ordering | Watch doesn't deliver ADDED events in sequence | Watch cache timing |
| preemption.go:1025 | RS never had availableReplicas | Deployment/RS controller `availableReplicas` not updated | RS controller fix |
| conformance.go:888 | ResourceClaim apply-patch+yaml | Server doesn't accept apply-patch+yaml content type | Content-type handling |
| aggregated_discovery.go:227 | CRD not in aggregated discovery | Dynamic CRD groups (#274 code was reverted) | Re-implement #274 |
| crd_publish_openapi.go:244 | CRD preserving unknown fields | CRD protobuf decoder | Protobuf fix |
| output.go:282 | Secret volume defaultMode+fsGroup | File permissions on secret volume | fsGroup handling |
| pods.go:556 | Pod generation != 1 | Pod generation field mismatch | Check generation handling |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 97 | ~400 | 441 | ~9% |
| 101 | 196 | 441 | 56% |
| 103 | 30 | 76 | 60% |
| 104 | 36 | 441 | ~92% |
| 105 | 7 | ~50/441 | ~86% (test in progress) |

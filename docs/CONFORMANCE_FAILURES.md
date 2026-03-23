# Full Conformance Failure Analysis

**Last updated**: 2026-03-23 (round 62 — CRITICAL pod IP fix deployed)

## CRITICAL FIX: Pod IP resolution
get_pod_ip was looking at any matching container, but main containers
use --net=container:pause and have no IP. Fixed to look at pause
container specifically. This likely fixes 20+ test failures that depend
on pod connectivity (webhooks, services, DNS, etc).

## 68+ fixes across 67 commits this session
Every known failure category has been investigated and most have fixes.

## Remaining known issues:
- Watch closed (1 test)
- kubectl create validation (protobuf parse error)
- CRD decode error (empty response)
- Cgroup CPU weight
- Some container exec output issues

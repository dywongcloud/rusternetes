# Conformance Issue Tracker

**Round 92**: 10 PASS, 3 FAIL so far | **123 fixes deployed** | Huge improvement!

## Round 92 failures

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:786 | watch closed before timeout | Watch stream closes — investigating with added logging |
| 2 | webhook.go:520 | webhook not ready | Liveness fix deployed — webhook pod may still not start |
| 3 | service_cidrs.go:170 | servicecidrs "kubernetes" not found | Need default ServiceCIDR resource |

# Conformance Issue Tracker

**Round 92**: 13 PASS, 9 FAIL so far | **124 fixes deployed** | 59% pass rate

## Round 92 failures

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:786 | watch closed before timeout | Watch stream investigation — added logging |
| 2 | webhook.go:520 | webhook not ready | Webhook pod may not start — investigate |
| 3 | service_cidrs.go:170 | ServiceCIDR "kubernetes" not found | **FIXED** pending deploy |
| 9 | kubelet_etc_hosts.go:97 | Duplicate mount /etc/hosts | **FIXED** pending deploy |
| 4 | init_container.go:440 | init container timeout | Needs investigation |
| 5 | kubectl.go:1130 | failed creating pod | kubectl pod creation error |
| 6 | runtime.go:158 | unknown | Needs investigation |
| 7 | predicates.go:1102 | context deadline exceeded | Scheduling/taint issue |
| 8 | watch.go:409 | watch notification timeout | Watch history replay issue |
| 9 | kubelet_etc_hosts.go:97 | pod failed in 4s | Pod goes to Failed phase immediately |

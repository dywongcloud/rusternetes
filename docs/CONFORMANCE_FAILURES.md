# Conformance Issue Tracker

**Round 92**: 13 PASS, 12 FAIL | **126 fixes** | 52% pass rate

## Failures

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:786 | watch closed | Watch stream investigation |
| 2 | webhook.go:520 | webhook not ready | Container starts but probe fails initially |
| 3 | service_cidrs.go:170 | ServiceCIDR not found | **FIXED** pending deploy |
| 4 | init_container.go:440 | init container timeout | Needs investigation |
| 5 | kubectl.go:1130 | pod creation failed | kubectl error |
| 6 | runtime.go:158 | unknown | Needs investigation |
| 7 | predicates.go:1102 | scheduling deadline | Scheduling resource check |
| 8 | watch.go:409 | watch notification timeout | Watch history replay |
| 9 | kubelet_etc_hosts.go:97 | duplicate /etc/hosts | **FIXED** pending deploy |
| 10 | taints.go:489 | pods not evicted | **FIXED** pending deploy — NoExecute taint eviction |
| 11 | aggregator.go:359 | apiserver deployment not ready | Container exits code 1 (needs local etcd) |
| 12 | table_conversion.go:167 | unknown | Needs investigation |

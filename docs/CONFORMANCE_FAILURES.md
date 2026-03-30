# Conformance Issue Tracker

**Round 115** | IN PROGRESS | 72/441 done | 42 passed, 26 failed (61.8%)

## Fixes Committed This Session (not yet deployed)

| Fix | Commit | Impact |
|-----|--------|--------|
| emptyDir umask 0000 for file permissions | aad60bc | Fixes `-rwxr-xr-x` → `-rwxrwxrwx` |
| WebSocket exec initial stdout frame | 5e1b78c + aad60bc | Fixes channel 3 before channel 1 |
| StatefulSet scale down one-at-a-time | 6d625c9 | Already deployed R115 |

## Code Bugs (13 in Round 115)

| Test | Error | Status |
|------|-------|--------|
| EmptyDir (root,0777,default) | File perms 0755 not 0777 | FIXED — umask 0000 wrapper (aad60bc) |
| WebSocket exec channel order | Channel 3 before channel 1 | FIXED — initial stdout frame (aad60bc) |
| EndpointSlice API operations | Expected | Need investigation |
| Pod InPlace Resize | Resize state | Complex feature |
| StatefulSet rolling update | Revision mismatch | Need investigation |
| InitContainer RestartNever fail | Expected | Need investigation |
| FieldValidation duplicate | Wrong format | Fix deployed but may need rebuild |
| Job FailIndex | Completion timeout | Fix deployed but still failing |
| Service endpoints latency | Missing selector field | Serialization issue |
| VAP validate Deployment | Denied | Fix deployed but still failing |
| HostPort pod2 timeout | Pod not starting | Timing |
| kubectl replace | Error running replace | Protobuf/content type |
| CSR PATCH | Request rejected | PATCH handling |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 110 | 158 | 441 | 64.2% |
| 115 | 26 | 72/441 | 61.8% (in progress) |

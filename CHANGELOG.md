# Changelog

All changes to this project will be documented in this file.

This project follows Semantic Versioning.

---

## [0.2.0] - 2026-02-18

### Added

#### Language Features
- `if / else` control flow support
- `if guard(x) { ... } else { ... }` conditional safety pattern
- Event definition syntax (`event { ... }`) mapped to struct layout

#### Map Improvements
- Dynamic ring buffer sizing (no longer hardcoded)
- `map.<method>` syntax:
  - `update`
  - `reserve`
  - `submit`
- Improved hash map usage patterns

#### Context Helpers
- `get_pid_tgid`
- `get_uid_gid`
- `get_current_comm`
- `get_current_task`

#### Time Helpers
- `get_ktime_ns`

#### Memory Probe Helpers
- `probe_read_user_str`
- `probe_read_kernel_str`

#### Examples
- Multiple tracepoint examples:
  - exec filename capture (ring buffer)
  - fork counter
  - exit counter
  - execve counter

---

### Improved

- Compiler error handling system
- Structured diagnostics
- Safer pointer handling patterns
- IR lowering stability
- Ring buffer implementation flexibility

---

### Fixed

- Minor parsing issues
- Guard handling edge cases
- Internal map handling consistency

---

## [0.1.0] - 2026-01-28

### Added
- Initial preview release
- Basic tracepoint support
- Hash map and init ebpf sections definitions
- Minimal example programs

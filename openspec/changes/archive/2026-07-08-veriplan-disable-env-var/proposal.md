## Why

When veriplan is integrated into a Justfile or CI pipeline, there's no way to temporarily disable it without editing the configuration. Users need a runtime escape hatch — set an environment variable, veriplan prints a warning and exits 0. This is useful for debugging CI failures, working around transient issues, or quickly testing changes without waiting for verification.

## What Changes

- Add `VERIPLAN_DISABLE` environment variable check at the top of `main()`
- If set to a truthy value (non-empty, not `0`/`false`/`no`), print a warning to stderr and exit 0
- All other execution paths are unchanged

## Capabilities

### New Capabilities

- `veriplan-disable`: Environment variable to disable veriplan at runtime

### Modified Capabilities

- *(none)*

## Impact

- **One file changed**: `src/main.rs` — add ~5 lines at the top of `main()`
- **No new dependencies**
- **No runtime impact when unset**

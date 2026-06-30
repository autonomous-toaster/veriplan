## Why

The YAML-aware config merge was implemented in commit baf172b but has no tests. Without tests, regressions in the merge logic (context appending, rule deduplication, cross-tool coexistence) go undetected. Groundcontrol's identical fix has 6 tests — veriplan has 0.

## What Changes

- Add unit tests for `merge_config()` covering: fresh file creation, idempotent re-run, existing content preservation, context appending, rule deduplication
- Add unit tests for `yaml_merge()` covering: context append, rule dedup by exact match
- No changes to production code — tests only

## Capabilities

### New Capabilities

*(none — no new capabilities, tests only)*

### Modified Capabilities

*(none — no spec-level behavior changes)*

## Impact

- **src/main.rs**: Add `#[cfg(test)] mod tests` block with 6 test functions
- **Cargo.toml**: No new dependencies (tempfile already present)

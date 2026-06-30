## Context

The YAML-aware config merge (committed in baf172b) mirrors groundcontrol's fix exactly. Groundcontrol has 6 tests covering the merge logic; veriplan has none. The `merge_config`, `yaml_merge`, `create_fresh_config`, and `veriplan_rules` functions are all testable via `tempfile` and `serde_yaml` — no mocking needed.

## Goals / Non-Goals

**Goals:**

- Add tests for `merge_config()` covering: fresh file, idempotent re-run, existing content preservation, rules content
- Add tests for `yaml_merge()` covering: context append, rule dedup by exact match
- All tests use `tempfile` for isolated filesystem access

**Non-Goals:**

- No changes to production code
- No new dependencies
- No integration tests (unit tests only)

## Decisions

**Decision 1: Mirror groundcontrol test structure**

Tests follow the same pattern as groundcontrol's `#[cfg(test)] mod tests` block. Each test creates a temp directory, exercises the function, and asserts on file content or YAML value structure.

**Decision 2: No mocking**

All functions operate on `Path` or `serde_yaml::Value` — no I/O mocking needed. `tempfile::tempdir()` provides isolated filesystem access.

## Risks / Trade-offs

- **[Test coverage gap]** Tests cover the merge functions but not the full `run_init` command. Mitigation: `merge_config` is the core logic; the CLI wrapper is trivial.

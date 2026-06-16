## Why

Running veriplan currently requires explicitly naming a single change (e.g., `veriplan check my-change`). This is inconvenient in CI and when iterating: you must know the change name, run it per-change, and there's no way to verify all active changes at once. Additionally, veriplan can only run from the project root and only against its own openspec directory — there's no way to point it at another project's openspec folder. This limits CI integration and cross-project verification.

## What Changes

- **`veriplan check` with no arguments**: auto-detect the project's openspec directory (look for `./openspec/changes/` in CWD), find all changes not in `archive/`, and verify each one in sequence.
- **`veriplan check <path>`**: if the argument is a directory path (not a change name), treat it as a project root, auto-detect openspec inside it, and verify all active changes.
- **`--format openspec` flag**: add an explicit `--format openspec` argument to `veriplan check`. This is a no-op for now (openspec is the only format) but establishes the pattern for future format support (speckit, custom plans, etc.).
- **Change name vs directory disambiguation**: detect whether the argument looks like a path (contains `/`, exists as a directory, has `openspec/` inside) vs a change name.

## Capabilities

### New Capabilities

- `auto-detect-changes`: detect active openspec changes from the project root and verify them all
- `project-path`: accept an external project directory and auto-detect its openspec changes
- `format-extensibility`: add `--format openspec` flag with a format registry pattern for future backends

### Modified Capabilities

- *(none — existing capabilities unchanged)*

## Impact

- CLI: `veriplan check` gains two new argument modes (no-arg and directory path)
- CLI: `--format` flag added with `openspec` as the only valid value
- Checker: `verify` function needs to support running against multiple changes
- Parser: needs to discover changes directory from an arbitrary project root
- No breaking changes: existing `veriplan check <change-name>` continues to work

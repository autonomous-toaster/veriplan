## Why

Veriplan's convertibility check (Phase 1) validates task structure and requirement references, but only checks for exact task ID matches (e.g., "T4.2"). It cannot detect when a requirement uses vague natural language like "the migration step" instead of the explicit task ID "T2.1". This means specs can pass convertibility checks but still be unverifiable because the NL-to-task-ID mapping is ambiguous.

The `groundcontrol` crate provides a rule-based grounding engine that matches NL requirement statements against a plan's task vocabulary using keyword matching + positional heuristics. Integrating it as a library dependency adds a spec quality gate that catches fuzzy mismatches before they reach the model checker.

## What Changes

- Add `groundcontrol` as a git dependency in `Cargo.toml`
- Create a new `grounding` module that wraps groundcontrol's `RuleGrounder` and builds a `Signature` from `PlanIR`
- Add a grounding check to the convertibility pipeline (between T4.2 and T4.4) that grounds each requirement's SHALL statement against the plan's task vocabulary
- Populate the existing `PatternUngrounded` `ConstraintCategory` variant with grounding results
- Surface grounding results in the convertibility report (blockers for ungroundable/ambiguous, warnings when relaxed)
- Fold grounding rules into `veriplan bootstrap` config generation
- Add strictness profile support for grounding severity

## Capabilities

### New Capabilities

- `grounding-check`: NL-to-task-ID grounding quality gate for requirement statements

### Modified Capabilities

- `convertibility-check`: Add grounding check between T4.2 and T4.4; populate `PatternUngrounded` category

## Impact

- **New dependency**: `groundcontrol` git repo at `https://github.com/autonomous-toaster/groundcontrol`
- **New module**: `src/grounding/` — wraps groundcontrol API, builds Signature from PlanIR
- **Modified module**: `src/checker/convertibility.rs` — add grounding check to pipeline
- **Modified module**: `src/checker/checks.rs` — add grounding check functions
- **Modified module**: `src/cli.rs` — grounding severity integrated into strictness profile
- **Modified module**: `src/input/loader.rs` or bootstrap — grounding rules in config
- **No breaking changes**: Grounding is additive; existing behavior preserved when no grounding issues found

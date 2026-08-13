## Why

The primary consumer of `veriplan check` is an AI assistant that reads the non-zero-exit output, edits the spec, and revalidates. Today that output is not built for an assistant: default JSON drops blockers entirely (shows `convertible: false` with `violations: []` and no actionable detail), convertibility blockers and model-check violations are two incompatible shapes, verbosity changes *what* is shown differently in human vs JSON, and the `fix` field is heterogeneous prose (some fixes are mechanically computable, most are vague). Research on LLM verify-repair loops (VeriHarness, self-reflective APIs, VRpilot, ThinkRepair) converges on one answer: a validator should return findings with **location + observed + expected replacement + a machine-applicability tier**, consistently across formats. steve — already a veriplan dependency — implements exactly this contract (`rule`/`severity`/`message`/`replacement`/`fixability`/byte-span), and veriplan's prose layer currently *throws away* `fixability`, `replacement`, `start`, and `end` at the boundary. This change adopts steve's proven linter contract for all of veriplan's output, so an assistant can reliably act on every finding.

## What Changes

- **Unify the output contract**: introduce one canonical `Finding` shape (rule id, severity, location with line/column/byte span, message, suggestion, optional `replacement`, and a `fixability` tier) that replaces the incompatible `CheckItem`/`Violation`/`ProseFinding` surfaces. All three sources (convertibility, model-check, prose) emit the same shape.
- **Fix the default-JSON gap** (**BREAKING**): `findings` are always present in default JSON and default human output, regardless of verbosity. Verbatim fields that used to gate on `--verbose` (rephrase directives, summaries) remain verbose-only, but the actionable findings never vanish. The top-level JSON shape changes from `convertibility_report`/`violations` to a single `findings[]` array.
- **Structured `kind` and `op`**: a curated `kind` enum (stable, assistant-switchable) derived from the real `check`/`category` values, and an `op` enum (split_requirement, rename_task, replace_body, etc.) expressing the application intent. `kind` is orthogonal to `severity` (severity stays strictness-mutable); `op` drives `--fix` eligibility.
- **Carry deterministic replacements**: emit `replacement[]` (with `op`) on findings where the edit is computable (grounding multi-keyword split, duplicate task id). Adopt steve's `replacement` + `fixability` for prose `Local` findings.
- **Fill coverage gaps**: give `Global` and `FixedTime` model-check violations a `suggest_fix` (currently `_ => None`); surface the three `non_formalizable` subtypes (`bare_capability`/`vague_action`/`vague_quality`) as distinct `kind`s instead of one `non_formalizable`; add steve's `Local` `SlopWord` rule to the curated prose set so prose findings can be machine-applicable.
- **Group by `kind` in default human output**: identical findings collapse to "N× <kind>: <rephrase>" with one location; `verbose` expands all. Automatic (no flag).
- **`veriplan check --fix`** (applies only `op ∈ {mechanical}` findings, i.e. byte-recoverable edits), with revalidation after each application; conservative about structural edits to avoid the over-repair risk documented in verify-repair-loop research.

## Capabilities

### New Capabilities

- `output-contract`: The unified `Finding` output contract for `veriplan check` — canonical shape, `kind`/`op` enums, `fixability` tiers, grouping, verbosity semantics, and `--fix` applicability. This is the reporting surface shared by convertibility, model-check, and prose.

### Modified Capabilities

- `convertibility-check`: CheckItems become `Finding`s with `kind`/`op`/`replacement`; the three `non_formalizable` subtypes become distinct kinds; fixes carry `fixability`.
- `model-check`: Violations become `Finding`s; `Global` and `FixedTime` violations get a `suggest_fix`.
- `grounding-check`: `grounding_ambiguous_multi_keyword` emits a structured `op: split_requirement` with concrete `replacement[]` bodies.
- `prose-guidance`: Pass through steve's `fixability`, `replacement`, `start`/`end`, `ste_rule`; add `SlopWord` to the curated set; prose findings are tagged advisory.

## Impact

- `src/ir/mod.rs`: add `Finding` + `kind`/`op`/`fixability` types (or adopt steve's `Finding`).
- `src/checker/checks.rs`: tag each `CheckItem` with `kind`/`op`/`fixability`; split `non_formalizable` subtypes.
- `src/checker/bfs.rs`: add `Global`/`FixedTime` `suggest_fix` arms.
- `src/grounding/mod.rs`: emit `op: split_requirement` + `replacement[]`.
- `src/prose/mod.rs`: carry through steve's structured fields; add `SlopWord` to curated rules.
- `src/annotator/mod.rs`: unify `format_human`/`format_json` on `findings[]`; always-present findings; grouping.
- `src/cli.rs` / `src/main.rs`: add `--fix`; update exit-code/verbosity handling.
- `src/lsp/*`: **BREAKING** — LSP consumers of `CheckItem.fix`/`suggested_fix` are updated or removed (LSP is being decommissioned).
- Tests: update the 24 test files that construct `CheckItem`/`Violation`.

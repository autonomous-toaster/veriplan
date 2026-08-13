# Design: Structured Finding Output for veriplan

## Context

See `proposal.md` — Why. The problem is that `veriplan check` output is not built for its primary consumer (an AI assistant that reads non-zero-exit output, edits the spec, revalidates). Today: default JSON drops blockers, convertibility `CheckItem`s and model-check `Violation`s are incompatible shapes, verbosity changes what's shown differently per format, and `fix` is heterogeneous prose.

Key realization from exploration: **steve (already a veriplan dependency) implements exactly the contract needed.** Its `Finding` struct (`rule`/`severity`/`message`/`suggestion`/`start`/`end`/`replacement`/`fixability`) is a working, serialized linter contract, and its CLI `--json` output is the target shape. veriplan's `prose.rs` currently copies only 6 of steve's 11 fields, discarding `fixability`, `replacement`, `start`, `end`, `column`, and `ste_rule`. This design adopts steve's contract and extends it to veriplan's own checks.

## Goals / Non-Goals

**Goals**
- One canonical `Finding` shape across convertibility, model-check, and prose.
- Actionable, always-present findings in both default JSON and human output.
- A curated `kind` (problem) and `op` (remedy) vocabulary that is stable, assistant-switchable, and orthogonal to strictness-mutable `severity`.
- A `fixability` tier driving `--fix` eligibility, mirroring clippy's `MachineApplicable`/`MaybeIncorrect`.
- Deterministic `replacement` on findings where the edit is computable.

**Non-Goals**
- No changes to the convertibility/model-check *verdicts* themselves (same findings, same severities — only the shape changes).
- No `--fix` for structural/judgment edits (those remain suggestions).
- Not extending the LSP (it is being decommissioned; consumers there are updated or removed, not enhanced).
- Not changing steve itself; veriplan consumes its existing contract.

## Decisions

### D1: Adopt steve's `Finding` field set as the base contract

Introduce a veriplan `Finding` type mirroring steve's shape: `kind`, `severity`, `file`, `line`, `column`, `start`, `end`, `message`, `suggestion`, `replacement`, `fixability`, plus veriplan-specific `requirement_id`/`element` and `advisory`.

**Why:** steve already proves this schema end-to-end (it serializes to JSON). Reusing it (a) de-risks the schema, (b) lets prose pass-through be a near-trivial struct mapping, (c) gives a consistent mental model. **Alternative considered:** a bespoke veriplan-only schema — rejected because it duplicates steve's proven shape and complicates the prose boundary.

### D2: `kind` and `op` as separate curated enums; `severity` stays a projection

`kind` = the problem (from `check`/`category` values, e.g. `grounding_multi_keyword`, `violation_sequential`, `bare_capability`, `prose_slop_word`). `op` = the remedy (e.g. `split_requirement`, `rename_task`, `replace_body`). `severity` remains a function of strictness profile (`strictness_severity`), independent of `kind`.

**Why:** the code already makes `severity` strictness-mutable (`pattern_ungrounded`/`no_tasks`/`no_requirements` shift across profiles). If `kind` encoded severity, it'd be unstable. Splitting `kind` (stable) from `severity` (projection) is the only way to keep a switchable vocabulary. `op` is separate because two `kind`s may share one `op`, and `op` is what drives `--fix`.

### D3: `fixability` tier per finding; `--fix` filters on it

`fixability ∈ {local, structural}` (steve's names) or clippy's `{machine_applicable, maybe_incorrect}`. Only `local`/`machine_applicable` findings are auto-appliable. `--fix` applies those, revalidates, and leaves the rest as suggestions.

**Why:** this is the convergence of three independent sources — steve's `Fixability`, clippy's applicability levels, and the verify-repair-loop research (VRR-Stop: repair can damage correct plans, so be conservative). **Conservative defaults:** `split_requirement` is `structural` (not auto-applied blindly) because a wrong split is worse than no split; only byte-recoverable edits like `SlopWord` (with replacement) and `rename_task` are `local`.

### D4: `replacement` distinguishes span edits from structural ops

steve's model is a byte-span + single `replacement` string. veriplan's highest-value fix (`split_requirement`) is *structural* (one requirement → two blocks). Encode both: span+`replacement` for `local` edits, and an `op` + `replacement[]` (list of before/after bodies) for structural ops. `split_requirement` carries `replacement[]` but is `structural` (assistant applies, revalidates).

**Why:** a single-string byte replacement cannot express "split into N blocks." The hybrid (span for local, op+array for structural) mirrors clippy exactly — `MachineApplicable` for safe local edits, `MaybeIncorrect` for edits needing a rewrite.

### D5: Findings always present; `--verbose` adds summaries, never removes findings

Both default JSON and human output include the full `findings[]`. `--verbose` gates only supplementary lists (rephrase directives, constraint summaries, category breakdown) and expands grouped findings.

**Why:** the confirmed bug is that non-verbose JSON drops blockers entirely. The research (VeriHarness: feedback without expected-alternatives ≈ raw diagnostic) says the findings *are* the value; verbosity must add information, not change the core shape.

### D6: Group by `kind` in default human output (automatic)

Identical findings collapse to "N× <kind>: <rephrase>" with one representative location; `verbose` expands all. Always on, no flag.

**Why:** anti-bloat *and* better for the assistant — it signals "this is one class of fix, apply it everywhere," which reduces round-trips and over-repair risk (VRR-Stop).

### D7: Prose findings carry steve's structured fields; `SlopWord` added to curated set

Pass through `fixability`, `replacement`, `start`/`end`, `column`, `ste_rule` from steve. Add `SlopWord` (a steve-`Local` rule) to the spec.md curated set so prose can produce machine-applicable findings.

**Why:** F1 (pass-through) is nearly free and unlocks steve's already-computed replacements. F2 (SlopWord) makes the prose `--fix` story real. The `SLOP_WORDS` table replaces AI-slop ("leverage→use") without false-flagging OpenSpec vocabulary (SHALL/MAY/GIVEN/WHEN/THEN are not slop). steve's `ste_rule` (official ASD-STE100 number, e.g. "5.2") is a valuable citable reference for "excellent spec" guidance.

## Risks / Trade-offs

- **Formatting-outruns-correctness** → If the redesign makes a *wrong* diagnosis easier to apply, it amplifies harm. **Mitigation:** add a gold-standard corpus of real good/bad OpenSpec changes as an acceptance gate for F5 — the new output format must not change verdicts (same findings, same severities, re-shaped only). This converts the biggest unknown-unknown into a testable known.
- **Structural edits auto-applied badly** → A wrong `split_requirement` breaks a previously-valid requirement. **Mitigation:** `split_requirement` is `structural` (not auto-applied); `--fix` applies one op at a time with revalidation. Only byte-recoverable edits are `local`.
- **JSON breaking change for existing consumers** → `convertibility_report`/`violations` top-level keys change to `findings[]`. **Mitigation:** explicit **BREAKING** in proposal; version the JSON shape if any retained consumer needs compat. The assistant is the primary consumer and is being retargeted anyway.
- **Severity-mutation matrix not exhaustively catalogued** → only `pattern_ungrounded`/`no_tasks`/`no_requirements` shift per `strictness_severity`, but `apply_strictness` may drain others. **Mitigation:** add a test asserting `kind` stability across all three strictness profiles for every `kind` (spec: "kind stable across strictness profiles").
- **LSP timeline** → F5 touches LSP consumers. **Mitigation:** update or remove LSP consumers in the same change; the LSP is being decommissioned.

## Migration Plan

1. Land the `Finding` type + `kind`/`op`/`fixability` in `ir/mod.rs` alongside (not replacing) the existing `CheckItem`/`Violation` initially, so the checker can emit both during transition.
2. Convert the annotator (`format_human`/`format_json`) to render `findings[]` from the new type; keep old field names temporarily behind a flag if needed.
3. Fill coverage gaps (Global/FixedTime fix, `non_formalizable` subtypes, SlopWord) — independent of the shape change.
4. Wire prose pass-through (F1/F2).
5. Add `--fix` (filters on `fixability`/`op`), revalidating after each application.
6. Update/remove LSP consumers; remove transitional compat.
7. **Rollback:** because the change is mostly in the rendering/annotator layer and the checker verdicts are unchanged, reverting to the old formatters restores prior behavior. The `Finding` type can coexist with old types during the transition.

## Open Questions

- **Exact curated `kind`/`op` enum membership** for prose: which steve rules reach output per artifact, and whether to namespace prose kinds as `prose_*` vs keep them under steve's slashed ids. The spec requires prose findings to carry `fixability`/`replacement`/`ste_rule`, but the exact namespace convention can be finalized during implementation without changing specs or tasks.
- **JSON versioning**: whether to add a top-level `"version"` field to the findings envelope. The spec fixes the shape but not versioning; can be decided when the breaking-change migration is implemented.

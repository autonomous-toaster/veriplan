# Tasks: Structured Finding Output

Reference: `design.md` (D1-D7), specs in `specs/`. Each task is small enough for one session.

## 1. Core Finding type

- [x] 1.1 Add `Finding` struct to `src/ir/mod.rs` with fields: `kind`, `severity`, `file`, `line`, `column`, `start`, `end`, `message`, `suggestion`, `replacement`, `fixability`, `requirement_id`, `advisory`. Add `Fixability` enum (`local`/`structural`) and `Op` enum (`split_requirement`, `rename_task`, `replace_body`, `add_task_reference`, `remove_requirement`, `add_scenario_step`, `fix_reference`, `add_temporal_keyword`, `informational_only`, `none`).
- [x] 1.2 Add `Kind` enum enumerating the curated problem vocabulary (from `check`/`category` values): `no_tasks`, `duplicate_task_id`, `no_requirements`, `no_phase_grouping`, `no_rfc2119_keyword`, `no_rfc2119_any`, `bad_task_reference`, `bare_capability`, `vague_action`, `vague_quality`, `unknown_non_formalizable`, `pattern_ungrounded`, `no_formalizable`, `grounding_multi_keyword`, `grounding_ambiguous`, `grounding_ungroundable`, `scenario_no_when`, `scenario_no_then`, `then_no_shall`, `task_not_covered`, `low_diversity`, `may_requirement`, `informational_requirement`, `violation_sequential`, `violation_concurrent`, `violation_conditional`, `violation_exclusive`, `violation_global`, `violation_fixed_time`, plus `prose_*` variants for steve rules.
- [x] 1.3 Add a `kind_of(check_or_category: &str) -> Kind` mapping function and a `Severity`-independent accessor so `kind` never encodes severity.

## 2. Coverage gaps (independent of shape change)

- [x] 2.1 In `src/checker/bfs.rs` `suggest_fix`, add `Global` and `FixedTime` match arms so model-check violations of those categories carry a suggested fix (currently `_ => None`).
- [x] 2.2 In `src/checker/checks.rs` `check_classifiability`, key the `check` off the `VagueDiagnosis` subtype (BareCapability → `bare_capability`, VagueAction → `vague_action`, VagueQuality → `vague_quality`, else `unknown_non_formalizable`) instead of collapsing all into `non_formalizable`.
- [x] 2.3 Add tests asserting `Global`/`FixedTime` violations produce a suggestion and that the three non-formalizable subtypes map to distinct kinds.

## 3. Prose pass-through (F1/F2)

- [x] 3.1 Extend `ProseFinding` in `src/prose/mod.rs` to carry `fixability`, `replacement`, `start`, `end`, `column`, and `ste_rule`.
- [x] 3.2 In `check_snippet`, copy these fields through from steve's `Finding` (currently only `severity`/`rule`/`message`/`suggestion`/`snippet`/`line` are copied).
- [x] 3.3 Add steve's `SlopWord` to the curated rule set for `spec` artifact in `rules_for`, so prose can produce `local`/machine-applicable findings.
- [x] 3.4 Add tests: SlopWord with replacement is `local`; SlopWord without replacement (e.g. "robust") is `structural`; prose fields (fixability/replacement/ste_rule) survive to the `CheckItem`-conversion boundary.

## 4. Emit Findings from all three sources

- [x] 4.1 In `src/checker/checks.rs`, attach `kind`/`op`/`fixability` to each `CheckItem` as it is created (map via `kind_of`; tag machine-applicable only for `duplicate_task_id`).
- [x] 4.2 In `src/grounding/mod.rs`, for `grounding_ambiguous_multi_keyword`, populate `op: split_requirement` and a structured `replacement[]` with the split bodies (already computed in `fix`). Mark it `structural` (not auto-applied).
- [x] 4.3 In `src/checker/spin.rs`/`spin_rs.rs`/`bfs.rs`, attach `kind` (violation_*) and `op` to each `Violation`.
- [x] 4.4 In the prose→report boundary in `src/checker/mod.rs` (`verify_with_strictness`), carry `fixability`/`replacement`/`ste_rule` onto the emitted `CheckItem` and tag prose findings `advisory`.

## 5. Annotator / rendering (unify + always-present + grouping)

- [x] 5.1 In `src/annotator/mod.rs`, add a `findings()` projection that flattens convertibility blockers/warnings/info and model-check violations into one `Vec<Finding>`.
- [x] 5.2 Rewrite `format_json` to always emit a top-level `findings[]` array (regardless of `--verbose`); move rephrase-directives/summaries under `--verbose` gating. **BREAKING**: drop top-level `convertibility_report`/`violations` keys in favor of `findings[]`.
- [x] 5.3 Rewrite `format_human` to always show findings at default verbosity and to group identical findings by `kind` ("N× <kind>: <rephrase>" with one representative location); `--verbose` expands grouped findings.
- [x] 5.4 Ensure `--verbose` adds only supplementary info (rephrase directives, category breakdown, constraint summaries) and never changes which core findings are present, in both formats.

## 6. --fix mode

- [x] 6.1 Add `--fix` flag to `src/main.rs`/`src/cli.rs` `Check` command.
- [x] 6.2 Implement `--fix` to apply only findings whose `op` is machine-applicable (`local`), using the structured `replacement` (e.g. `duplicate_task_id` rename, prose `SlopWord` with replacement); skip `structural`/judgment ops.
- [x] 6.3 Apply edits one op at a time and revalidate the plan after each; emit a report of what was applied vs. left as suggestions.
- [x] 6.4 Add tests: `--fix` applies `local` ops and leaves `structural` findings as suggestions; a `split_requirement` finding is NOT auto-applied by `--fix`.

## 7. LSP consumers and cleanup

- [x] 7.1 Update or remove LSP consumers of `CheckItem.fix`/`Violation.suggested_fix` (`src/lsp/code_actions.rs`, `diagnostics.rs`, `state.rs`) to the new `Finding` shape, or remove them if the LSP is decommissioned in this change.
- [x] 7.2 Remove any transitional compat for the old `convertibility_report`/`violations` JSON keys after confirming the assistant consumer is updated.
- [x] 7.3 Update the 24 test files that construct `CheckItem {`/`Violation {` to the new `Finding` construction.

## 8. Acceptance gate

- [x] 8.1 Add a gold-standard corpus test (real good/bad OpenSpec changes) asserting the new output format produces the *same verdicts* (same findings, same severities) as before — only the shape changes.
- [x] 8.2 Add a test asserting `kind` stability across Strict/Moderate/Lax for every `kind` (severity may shift, `kind` must not).
- [x] 8.3 Add a test asserting default JSON and default human output describe the same set of `Finding`s.

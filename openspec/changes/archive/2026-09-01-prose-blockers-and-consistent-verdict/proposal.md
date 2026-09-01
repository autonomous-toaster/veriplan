## Why

A real session exposed two output-quality failures in `veriplan check` that actively misled the model:

1. **The verdict and the findings contradict each other.** `status_label()` derives "✓ VALID" from the model-check result (`convertible && valid == Some(true)`) alone, while prose findings live in a separate signal that never touches the verdict. A plan can print "✓ VALID" while simultaneously printing `[BLOCKER] prose_other`. The model had to reconcile a contradiction instead of trusting the output.
2. **Prose finding coordinates are snippet-relative but reported as file-absolute.** steve returns `line`/`start`/`end` relative to the single-line snippet it was given (a task description), but the annotator reports them as file coordinates. The model read `tasks.md:1, start:0, end:102` as file bytes 0–102 (task 1.1) when the real offender was task 5.1 — costing ~4 minutes and ~20 tool calls chasing a phantom blocker.

Underneath both is a severity-bucketing bug: a `"blocker"`-severity prose finding is pushed into `report.info` (so it displays as `[BLOCKER]` but never reaches the verdict or exit code).

## What Changes

- **Prose findings gate the verdict in Strict mode.** The two ambiguity-indicating prose rules — `OneInstructionPerSentence` and `PronounAmbiguity` — become real blockers in `Strict`. These are the rules that genuinely indicate ambiguity without fighting OpenSpec's required (passive, RFC 2119) grammar. The verdict and exit code derive from the same flattened `findings[]` set that is printed, so the label and the list can never disagree. Moderate/Lax keep prose advisory.
- **Fix severity bucketing.** A finding with severity `blocker` SHALL live in `report.blockers`, not `report.info`, so it reaches the verdict and exit code.
- **Report file-absolute coordinates.** Prose findings SHALL report the real file line and byte offset (snippet start offset added to steve's snippet-relative values), and the coordinate contract SHALL be documented in the spec.
- **Pre-commit inherits prose blockers.** A prose blocker in Strict blocks a commit, consistent with how pre-commit already treats blockers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `prose-guidance`: the "Report prose findings without blocking" requirement changes — `OneInstructionPerSentence` and `PronounAmbiguity` findings SHALL block in Strict mode, while remaining advisory in Moderate/Lax. The severity mapping requirement changes to mark these two rules as blockers in Strict.
- `output-contract`: the verdict SHALL be derived from the flattened `findings[]` set (no independent signal), and a `blocker`-severity finding SHALL be bucketed into `report.blockers`. The `Finding` coordinate contract SHALL be file-absolute.
- `strictness-profiles`: the prose severity mapping SHALL mark `OneInstructionPerSentence` and `PronounAmbiguity` as blockers in Strict.

## Impact

- `src/annotator/mod.rs` — `status_label()` derives the verdict from findings; `check_item_to_finding` coordinate handling.
- `src/checker/mod.rs` — `verify_with_strictness` buckets `blocker`-severity prose findings into `report.blockers`.
- `src/prose/mod.rs` — `check_prose` computes file-absolute coordinates; severity mapping for the four blocker rules.
- `src/cli.rs` / `src/main.rs` — exit-code logic reflects prose blockers in Strict and pre-commit.
- No change to the model-checking engine itself; this is output/verdict-layer only.

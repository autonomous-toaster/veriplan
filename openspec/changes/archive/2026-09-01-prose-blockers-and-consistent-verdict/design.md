## Context

See proposal.md for motivation. The current output layer has two independent signals: `status_label()` reads `convertible && valid` (model-check only), while prose findings are appended to `report.warnings`/`report.info` and never touch the verdict. Prose finding coordinates come from steve relative to a single-line snippet but are reported as file coordinates. A `"blocker"`-severity prose finding is pushed into `report.info` (severity string says `blocker`, bucket says `info`).

## Goals / Non-Goals

**Goals:**
- The verdict and exit code derive from the same flattened `findings[]` set that is printed — no independent signal, no contradiction.
- The four semantically-correlated prose rules block in Strict; Moderate/Lax stay advisory.
- Prose findings report file-absolute line and byte coordinates.
- A `blocker`-severity finding lives in `report.blockers`.

**Non-Goals:**
- Changing the model-checking engine or the grounding/convertibility logic.
- Changing which prose rules are *run* (the curated set stays as-is).
- Making prose block in Moderate or Lax.

## Decisions

### D1 — Verdict is a pure function of the flattened findings

`status_label()` SHALL compute the verdict from the same `findings()` projection that `format_human`/`format_json` print, not from `valid`/`convertible` in isolation. Concretely: if any finding has severity `blocker`, the status is not "✓ VALID". This makes contradiction structurally impossible because the label and the list share one source of truth.

**Why not keep `valid`/`convertible` as the source?** They are a *subset* of the findings (model-check + convertibility) and omit prose. Any verdict built from them can disagree with a printed prose blocker. Deriving from the full set closes the gap.

### D2 — Two prose rules block in Strict only

The two ambiguity-indicating rules `OneInstructionPerSentence` and `PronounAmbiguity` map to `Severity::Hard` in Strict (already the case today) and that `Hard` now means "blocks". Moderate/Lax keep prose advisory. This preserves the strictness ladder: Strict is the hard gate, Moderate is the soft gate.

**Why only two?** The other two originally considered — `PassiveVoice` and `Hedging` — were dropped after a self-check: OpenSpec's required grammar is inherently "SHALL be" passive and uses RFC 2119 "may". Flagging those as blockers would fight the mandated grammar and produce false positives on every legitimate spec. `OneInstructionPerSentence` (a two-instruction task is ambiguous to the grounder) and `PronounAmbiguity` (unclear refs blur task refs) genuine indicate ambiguity without colliding with OpenSpec grammar. The pure-style rules (`SynonymConsistency`, `SentenceLength`, `SlopWord`) stay advisory.

### D3 — Fix severity bucketing

In `verify_with_strictness`, a prose finding with severity `blocker` SHALL be pushed to `report.blockers`, not `report.info`. The current `if severity == "warning" { warnings } else { info }` branch is replaced with an explicit three-way match on severity.

### D4 — File-absolute coordinates

`check_prose` SHALL compute each snippet's start offset in its source file and add it to steve's snippet-relative `line`/`start`/`end` before emitting the `ProseFinding`. For task descriptions this means the task's actual line in `tasks.md` and the column within it. The `Finding` coordinate contract SHALL be documented as file-absolute in the spec.

**Why not keep snippet-relative?** The session proved snippet-relative coordinates are actively misleading — the model chased a phantom blocker for 4 minutes. File-absolute coordinates are what a user or model can act on directly.

### D5 — Pre-commit inherits prose blockers

`--pre-commit` already treats blockers as exit-1. With prose blockers real in Strict, a Strict pre-commit hook rejects commits on the four rules. This is consistent with the existing blocker contract and keeps "a blocker is a blocker everywhere".

## Risks / Trade-offs

- **Strict pre-commit becomes harsher** — ambiguous prose (two-instruction tasks, ambiguous pronouns) now blocks a commit in Strict. Mitigation: this is the intended contract (D5); users who want a soft gate run Moderate or Lax.
- **False positives on the two rules** — e.g. a pronoun that appears ambiguous but resolves well in context. Mitigation: the two rules are chosen specifically because they do not collide with OpenSpec's required grammar (unlike passive voice and hedging); a data-driven refinement (block only when the finding co-occurs with a grounding failure on the same element) is a possible follow-up but is out of scope here for determinism.
- **Coordinate computation needs snippet→file mapping** — `check_prose` must track where each snippet starts in its file. Mitigation: the plan IR already carries per-element source locations; the snippet start can be derived from the element's source span.

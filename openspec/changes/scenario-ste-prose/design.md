## Context

`check_prose` in `src/prose/mod.rs` currently checks three zones: requirement
bodies (full curated set), task descriptions (minimal set), and design/proposal
body paragraphs (light set). Scenario steps are **not** checked — `extract_shall_statement`
stops at the first `####` heading, so `req.statement` never contains scenarios.

`extract_scenarios` in `src/parser/helpers.rs` already parses scenario steps
into `Scenario` structs with `GIVEN`/`WHEN`/`THEN`/`AND` steps. This is the
reusable entry point.

Empirical probes (steve run directly on real scenario content) established:
- Full curated set on raw scenario → 1 false positive (PassiveVoice on
  "the plan SHALL be marked VALID").
- Safe subset (Pronoun/Synonym/SentenceLength) on raw + stripped → 0 findings.
- Safe subset on a genuinely vague scenario → 1 true positive (pronoun).

## Goals / Non-Goals

**Goals:**
- Apply a safe STE subset (PronounAmbiguity, SynonymConsistency, SentenceLength)
  to scenario step content.
- Strip `**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**` markers and code spans before
  checking, so the structured scaffolding doesn't produce noise.
- Keep findings advisory (rephrase directives), never blocking.
- Reuse `extract_scenarios` and the existing `ProseFinding` pipeline.

**Non-Goals:**
- Do NOT apply PassiveVoice or OneInstructionPerSentence to scenarios (proven
  false positives on legitimate state assertions).
- Do NOT change verdicts — prose findings stay advisory.
- No new `ConstraintCategory` variants.
- No changes to requirement-body / task / design / proposal checking.

## Decisions

**D1 — Safe rule subset for scenarios.**
`PronounAmbiguity`, `SynonymConsistency`, `SentenceLength`. Exclude
`PassiveVoice` (false-positives on "the plan SHALL be marked VALID") and
`OneInstructionPerSentence` (a THEN with two assertions is often legitimate).
Hedging is optional — RFC 2119 modals (SHOULD/MAY) are already excluded from
steve's hedging rule, so it adds little for scenarios.
*Rationale:* empirically proven — the safe subset produces 0 false positives on
real scenarios and catches genuine ambiguity.

**D2 — Strip scaffolding before checking.**
For each scenario step, remove the `**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**`
marker and inline code spans (`` `...` ``) and task IDs (T1.4) before feeding
steve. This mirrors the zone-scoping already applied to requirement bodies.
*Alternative considered:* feed raw steps to steve. Rejected — the markers
produce noun-cluster/topic-sentence noise (observed in probes).

**D3 — Reuse `extract_scenarios`.**
`check_prose` calls `extract_scenarios` on each requirement body to get the
steps, then checks each step's content with the safe subset. No new parser work.

**D4 — Advisory only.**
Scenario prose findings flow through the existing `ProseFinding` →
rephrase-directive pipeline, never contributing a blocker. Consistent with the
existing prose-guidance design (D5 from `veriplan-steve-prose-guidance`).

## Risks / Trade-offs

- **Narrow value**: scenario steps are already the most machine-checked part of
  the spec (grounding + RFC 2119 + task refs). The safe subset catches only
  pronoun/synonym/sentence-length issues, which are less common than in
  requirement bodies. This is a marginal improvement, not a critical fix.
- **False-positive risk**: mitigated by the safe subset (proven 0 findings on
  real scenarios) and by stripping scaffolding first.
- **Hedging excluded**: RFC 2119 modals are already excluded from steve's hedging
  rule, so including Hedging would add little; keeping it out avoids any residual
  risk. Can be revisited if demand emerges.

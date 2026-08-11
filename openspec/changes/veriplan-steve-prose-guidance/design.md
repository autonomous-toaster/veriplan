## Context

veriplan verifies that OpenSpec plans are convertible to a formal state machine (task IDs, temporal keywords, grounding) then model-checks them with SPIN. It has strong *semantic* ambiguity detection (grounding confidence) but no *lexical/stylistic* ambiguity detection. The `veriplan init` config nudges writers ("no vague verbs") but never enforces it.

`steve` is a separate project (path dependency at `../steve`) implementing ASD-STE100 writing rules as a Rust library. Running stock steve over veriplan's archived OpenSpec corpus yields ~95% noise — dominated by false positives from:
1. **veriplan's required grammar** — `shall` (RFC 2119) flagged as unapproved word, `**GIVEN**/**WHEN**/**THEN**` scenario scaffolding flagged as noun-clusters/topic-sentences, temporal predicates `BEFORE/AFTER/IF_THEN` flagged.
2. **software vocabulary** the aerospace STE100 dictionary forbids (`return`, `create`, `verify`, `PlanIR`, `Signature`, `ground`, `alias`).

Only ~5% of findings are genuinely useful, and that residue is exactly the *unambiguity* signal that co-occurs with grounding trouble: a passive requirement ("A .md file path SHALL be resolved") names no task agent, so it grounds poorly.

This change wires a **curated subset** of steve's rules into veriplan as a **soft advisory layer**, scoped to prose zones per artifact, mapped to veriplan's StrictnessProfile, and **correlated with grounding outcomes** so the AI assistant gets one actionable directive fixing both style and semantics.

## Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                      Prose-guidance in the veriplan pipeline               │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  OpenSpec change dir                                                       │
│  ├── tasks.md  ────────────┐                                               │
│  ├── specs/*/spec.md ──────┤  tree-sitter → PlanIR                          │
│  ├── design.md ────────────┤  (every element has SourceLocation)            │
│  └── proposal.md ──────────┘                                               │
│         │                                                                  │
│         ▼                                                                  │
│  ┌────────────────────────────────────┐                                   │
│  │  Prose-zone selector               │  veriplan slices prose zones       │
│  │  (per artifact, per element)       │  (or passes exclusion ranges to    │
│  │  spec: requirement bodies only     │   steve — keeps provenance)        │
│  │  tasks: descriptions               │                                     │
│  │  design/proposal: body paragraphs  │                                     │
│  └──────────────┬─────────────────────┘                                    │
│                 │ prose substrings (+ provenance)                          │
│                 ▼                                                          │
│  ┌──────────────────────────────┐     curated rules:                       │
│  │  steve (curated rule set)    │──▶  PassiveVoice, PronounAmbiguity,      │
│  │  SteBuilder per artifact     │      Hedging, OneInstructionPerSentence, │
│  │  .rule(...Off) on the rest   │      SynonymConsistency, SentenceLength  │
│  └──────────────┬───────────────┘                                          │
│                 │ steve findings (line/col/rule/severity)                  │
│                 ▼                                                          │
│  ┌──────────────────────────────┐                                         │
│  │  Strictness mapping          │  Strict→hard/soft, Moderate→soft,        │
│  │  (per StrictnessProfile)     │  Lax→info                                │
│  └──────────────┬───────────────┘                                         │
│                 ▼                                                          │
│  ┌──────────────────────────────┐  grounding outcomes per requirement      │
│  │  Correlation with grounding  │──▶ (Ungroundable / Ambiguous / Grounded) │
│  │  (same requirement)          │                                          │
│  └──────────────┬───────────────┘                                         │
│                 │ combined rephrase directive                              │
│                 ▼                                                          │
│  ┌──────────────────────────────┐                                         │
│  │  Convertibility report       │  prose findings are advisory —          │
│  │  (rephrase directives)       │  never a blocker                        │
│  └──────────────────────────────┘                                         │
│                                                                            │
│  Also surfaced via LSP diagnostics (existing pipeline).                    │
└────────────────────────────────────────────────────────────────────────────┘
```

## Decisions

### D1 — Curated rule set (data-driven)
Only six steve rules are enabled: PassiveVoice, PronounAmbiguity, Hedging, OneInstructionPerSentence, SynonymConsistency, SentenceLength. All others (notably DictionaryNotApprovedWord, NounCluster, TopicSentence, ConditionBeforeCommand, ListsForSequences, ModalLadder) are disabled because they flag veriplan's required grammar or technical vocabulary. This is the "it may not apply to all parts of the markdown" answer — applied at the rule level, not the doc level.

### D2 — Per-artifact scoping
- `spec.md`: full curated set on **requirement body paragraphs only** — skip scenario scaffolding (`**GIVEN**/**WHEN**/**THEN**` list items), inline code spans, and predicate keywords.
- `tasks.md`: minimal set (OneInstructionPerSentence, Hedging) on **task descriptions** — descriptions become grounding aliases, so terse/imperative style is preserved and only ambiguity is flagged.
- `design.md` / `proposal.md`: light set (PassiveVoice, PronounAmbiguity, Hedging) — these are more human-oriented; only clarity matters.

### D3 — Strictness mapping (reuses existing knob)
Prose finding severity follows veriplan's existing `StrictnessProfile`:
- `Strict`: PassiveVoice + OneInstructionPerSentence → hard; others → soft
- `Moderate`: all → soft
- `Lax`: all → info

### D4 — Correlation (the "killer feature")
When a steve style finding (passive/pronoun/hedging) and a grounding failure (`Ungroundable` / `Ambiguous`) hit the *same* requirement, emit ONE combined directive: "This requirement is passive AND ungrounded — name the agent as a task ID." This is stronger than either signal alone and guides the AI to a single rewrite that satisfies both the style rule and the convertibility gate. Runs in all modes including Lax (where ungrounded is info-only and the steve hint may be the sole weak-signal).

### D5 — Advisory only, never blocking
Prose findings never flip a plan to Blocking. Blocking remains the exclusive job of structural/semantic checks. This keeps steve a *guidance* layer, not a second gate.

### D6 — steve is a path dependency
`steve = { path = "../steve" }`. Two small additive steve changes are anticipated (both live in steve, reusable beyond veriplan):
- **Exclusion-range scoping**: steve accepts include/exclude line ranges so prose-zone scoping preserves line/column provenance (alternative: veriplan slices substrings, losing provenance).
- **Configurable max-sentence-length**: a spec requirement body naturally runs several clauses ("T2.1 SHALL complete BEFORE T3.2 SHALL run"); add a `TextKind` variant or builder-level config rather than the fixed 20-word procedural limit.

## Validation (dogfood against veriplan's own OpenSpec corpus)

We validated the curated configuration by running a dogfood example binary
(`steve/steve/examples/veriplan_dogfood.rs`) that builds a `Ste` with exactly
the curated rule set + strictness mapping from D1/D3, over veriplan's own
OpenSpec documents. Results (46 spec files, 13 tasks.md, 13 design.md,
13 proposal.md):

| Artifact  | Scoped rules (D2)              | Findings/file | Notes                              |
|-----------|--------------------------------|---------------|-------------------------------------|
| spec.md   | full curated set (6 rules)      | ~4.5          | passive 83, sentence-len 61, pronoun 44, hedging 11, synonym 6 |
| tasks.md  | OneInstruction + Hedging        | ~0.9          | 12 one-instruction findings only    |
| design+prop| passive + pronoun + hedging   | ~4.0          | passive 23, pronoun 80              |

Key empirical results:
1. **Noise eliminated.** Stock steve produced ~60+ findings/file (mostly
   false positives). The curated set produces ~4.5 findings/file, all from
   curated rules — zero dictionary / noun-cluster / topic-sentence noise.
2. **True positives confirmed.** Real requirements like
   `"A .md file path SHALL be resolved relative to the current working
   directory"` (input-resolution/spec.md) and `"The response SHALL be a
   Location"` (lsp-navigation/spec.md) are passive with NO task agent —
   exactly the correlation target in D4 (they also ground poorly).
3. **Scenario-step false positives exposed — D2 prose-zone exclusion is
   load-bearing.** The dogfood (which only varies the rule set per artifact,
   not the prose zones) still flags `**THEN**` scenario steps on the corpus:
   `"the plan SHALL be marked VALID"` (model-check/spec.md line 42),
   `"the plan SHALL be marked NOT CONVERTIBLE"` (line 68),
   `"t1_3 are done"` (line 26). These are scenario scaffolding that a
   full D2 prose-zone exclusion must skip. Without that exclusion, the
   curated rules still emit false positives on `**GIVEN**/**WHEN**/**THEN**`.
4. **Purpose prose yields genuine findings.** In the same spec, `"...pipeline
   — it runs after..."` (line 5) gets a real pronoun-ambiguity finding
   ("it" has ambiguous antecedent) and `check`/`verify` synonym drift —
   both legitimately serve the unambiguity goal.

This validates that the rule-set scoping (D1) and strictness mapping (D3)
work end to end, and that **D2 prose-zone exclusion must be implemented** in
veriplan (not just the rule set) to avoid false positives on scenario
scaffolding.

## Risks & Mitigations

- **steve dependency coupling**: veriplan now depends on `../steve`. Mitigate by making the steve changes additive (new builder methods / TextKind variants, no behavior change to existing callers) so steve's own tests and consumers are unaffected.
- **Noise reintroduction**: if a future curated rule proves noisy on real OpenSpec, it can be disabled per-rule via the same builder config — no structural change needed.
- **Provenance drift on substring slicing**: if veriplan slices prose and passes substrings, line/col shift. Prefer steve exclusion-range scoping (D6) to keep provenance exact.
- **Scenario-step false positives**: D2 prose-zone exclusion is required; without it, `**THEN**` steps like "the plan SHALL be marked VALID" are flagged. Mitigate by scoping steve input to requirement bodies / task descriptions only.

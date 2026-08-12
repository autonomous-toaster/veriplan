## Why

veriplan validates that OpenSpec plans are *convertible* to a formal state machine (structural/semantic checks: task IDs, temporal keywords, grounding). But it has no check on *prose clarity* — a requirement like "A .md file path SHALL be resolved relative to the current working directory" passes every convertibility check, yet is passive, names no task agent, and grounds poorly or not at all.

The `veriplan init` config already *nudges* writers ("no vague verbs", "avoid hedging") but never *enforces* it. There is a real gap: veriplan catches semantic ambiguity (bad task refs, missing temporal keywords) but not the lexical/stylistic ambiguity that *causes* grounding trouble.

We have `steve` (our other project) — an ASD-STE100 writing-rules linter. Empirically running stock `steve` over veriplan's own archived OpenSpec corpus produces ~95% noise (60+ findings per file), dominated by false positives: `shall` flagged as an unapproved word (41× in one spec), `**GIVEN**/**WHEN**/**THEN**` scenario scaffolding flagged as noun clusters / topic sentences, and software vocabulary the aerospace STE100 dictionary forbids. Only ~5% of findings are genuinely useful — and that useful residue is exactly the "unambiguity" signal (passive voice, pronoun ambiguity, hedging, one-instruction-per-sentence, synonym consistency) that *co-occurs with grounding failures*.

This change wires a **curated subset** of steve's rules into veriplan as a **soft advisory layer** scoped to the prose zones veriplan actually wants to check — never fighting veriplan's own required grammar — and correlates steve's style findings with grounding outcomes so the AI assistant gets a single, actionable rephrase directive that fixes *both* the style and the semantics.

## What Changes

- **New `steve` dependency** in veriplan (path dependency on `../steve`).
- **Curated rule set**: enable only a small subset of steve's rules (PassiveVoice, PronounAmbiguity, Hedging, OneInstructionPerSentence, SynonymConsistency, SentenceLength). All other steve rules are disabled — in particular DictionaryNotApprovedWord, NounCluster, TopicSentence, ConditionBeforeCommand, ListsForSequences, and ModalLadder, which fight veriplan's required grammar (RFC 2119 keywords, temporal predicates, GIVEN/WHEN/THEN scenarios).
- **Prose-zone scoping**: steve findings are computed only over the prose that veriplan actually wants to check, and scoped *per artifact*:
  - `spec.md`: full curated set on requirement body paragraphs only (skip scenario scaffolding + inline code + predicate keywords)
  - `tasks.md`: minimal set (OneInstructionPerSentence, Hedging) on task descriptions — because descriptions become grounding aliases
  - `design.md` / `proposal.md`: light set (PassiveVoice, PronounAmbiguity, Hedging)
- **Strictness-profile mapping**: steve finding severity is driven by veriplan's existing `StrictnessProfile` (Strict/Moderate/Lax), same knob grounding already uses. In **Lax** mode, prose findings are INFO only — but they still surface.
- **`veriplan init` config guidance**: `veriplan init` writes prose-guidance instructions into `openspec/config.yaml` (context + per-artifact rules) telling the author exactly where steve applies: active voice with named task agents in requirement bodies, one temporal constraint per SHALL, terse imperative task descriptions. It also documents that steve does NOT check scenario scaffolding or inline code.

- **Grounding correlation**: when a steve style finding (passive/pronoun/hedging) and a grounding failure (ungroundable / low-confidence) occur on the *same* requirement, veriplan emits a **combined rephrase directive**: "This requirement is passive AND ungrounded — name the agent as a task ID, e.g. 'T1.2 SHALL resolve ...'". This runs in all strictness modes, including Lax (where ungrounded is only INFO and the steve hint may be the sole weak-requirement signal).
- **steve-side changes** (in `../steve`), formalized in this change:
  - **Exclusion-range scoping**: steve accepts include/exclude line ranges so veriplan can scope prose zones (skip scenario scaffolding, inline code, predicate keywords) while preserving line/column provenance.
  - **Configurable max-sentence-length**: a spec requirement body naturally runs several clauses ("T2.1 SHALL complete BEFORE T3.2 SHALL run"); add a `TextKind` variant or builder-level config rather than the fixed 20-word procedural limit.

## Capabilities

### New
- `prose-guidance`: soft advisory findings from steve's curated rules, scoped to per-artifact prose zones and mapped to StrictnessProfile severity.

### Modified
- `check` (convertibility phase): additionally runs the steve prose check and correlates its findings with grounding outcomes to produce combined rephrase directives. Never adds blockers from prose findings.
- `grounding`: its per-requirement outcomes feed the correlation with steve findings.

## Impact

- veriplan gains a cheap, local, deterministic hint that predicts which requirements will ground badly — before or alongside the heavier grounding check.
- No new CLI surface: the feature rides the existing `--strictness` flag and the existing rephrase-directive / report / LSP diagnostic pipeline.
- steve findings are **advisory only** — they never block a plan. Blocking remains the sole job of veriplan's structural/semantic checks.
- **Validated by dogfood**: the curated config was run over veriplan's own OpenSpec corpus (46 specs, 13 tasks, 13 designs, 13 proposals) and steve's own corpus (29 specs, 12 tasks) via the generalized `steve/examples/openspec_prose.rs` example. Findings drop from ~60+ (stock steve noise) to ~4.5 per file, all from curated rules. Real passive+ungrounded requirements are caught (e.g. "A .md file path SHALL be resolved"); scenario `**THEN**`/`**WHEN**` steps are only avoided once the D2 prose-zone exclusion is implemented — that exclusion is a required, load-bearing part of this change.
- Requires `steve` to be reachable as a path dependency (`../steve`); steve changes (exclusion ranges, sentence-length config) are additive and do not change existing steve behavior or its own test suite.

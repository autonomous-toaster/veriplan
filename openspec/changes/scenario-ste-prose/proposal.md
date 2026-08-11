## Why

Scenario steps (`**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**`) are the executable
contract of a spec — they pin down the concrete behavior an implementation must
satisfy. But they are currently **excluded entirely** from STE prose checking
(`extract_shall_statement` stops at the first `####` heading). This means a
scenario step like "**THEN** the system SHALL respond appropriately" — a
genuinely ambiguous assertion that makes implementation verification ambiguous —
is never flagged.

The exclusion was motivated by a real false positive: steve's `PassiveVoice`
rule fires on legitimate state assertions like "**THEN** the plan SHALL be
marked VALID". But empirical testing shows this false positive is **specific to
PassiveVoice and OneInstructionPerSentence**, not to scenario content in general.

## What Changes

Apply a **safe subset** of STE rules to scenario step content, while keeping the
noisy rules (PassiveVoice, OneInstructionPerSentence) out.

For each scenario step:
1. Strip the `**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**` markers.
2. Strip inline code spans and task IDs (same zone-scoping as requirement bodies).
3. Run the **safe subset**: `PronounAmbiguity`, `SynonymConsistency`, `SentenceLength`.
   (Hedging is optional/low-value — RFC 2119 modals are already excluded from it.)

**Empirically proven** (via direct steve probes on real scenario content):
- Safe subset on raw scenario → **0 findings** (no false positives).
- Safe subset on stripped scenario → **0 findings** (no false positives).
- Safe subset on a genuinely vague scenario ("it SHALL respond appropriately")
  → **1 true positive** (pronoun ambiguity caught).
- Full curated set on raw scenario → **1 false positive** (PassiveVoice on
  "the plan SHALL be marked VALID") — confirms why PassiveVoice stays out.

## Capabilities

### New Capabilities
- `scenario-ste-prose`: Apply a safe subset of STE prose rules (PronounAmbiguity,
  SynonymConsistency, SentenceLength) to scenario step content, stripping the
  GIVEN/WHEN/THEN/AND scaffolding and code spans first.

### Modified Capabilities
<!-- none — this is a new capability; no existing spec-level behavior changes -->

## Impact

- `src/prose/mod.rs`: extend `check_prose` to also check scenario steps with a
  new safe rule subset; add a `rules_for_scenario()` helper (or similar).
- `src/parser/helpers.rs`: `extract_scenarios` already parses steps — reuse it;
  add marker/code-span stripping for the prose pass.
- `src/ir/mod.rs`: no `ConstraintCategory` change; findings flow through the
  existing `ProseFinding` / rephrase-directive pipeline.
- Tests: unit tests for the safe subset (no false positives on legitimate
  scenarios; true positive on vague scenario; PassiveVoice/OneInstruction stay
  out).
- **No behavior change to verdicts** — prose findings remain advisory
  (rephrase directives), never blocking.

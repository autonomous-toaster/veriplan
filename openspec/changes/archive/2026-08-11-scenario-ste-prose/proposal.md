## Why

Scenario steps (`**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**`) are the executable
contract of a spec — they pin down the concrete behavior an implementation must
satisfy. But they are currently **excluded entirely** from STE prose checking
(`extract_shall_statement` stops at the first `####` heading). This means a
scenario step like "**THEN** the system SHALL respond appropriately" — a
genuinely ambiguous assertion that makes implementation verification ambiguous —
is never flagged.

The exclusion was motivated by false positives. Empirical testing on **196 real
scenario steps** from sibling projects (steve, groundcontrol, shield, spin-rs)
found two noise sources in steve, both now fixed:

1. `PronounAmbiguity` miscounted **verbs as noun antecedents** ("the engine
does not flag it" flagged "it" as ambiguous between "engine" and "flag" — a
verb). Fixed in steve (`b4e79b5`): candidate nouns now require a preceding
determiner/article or first-word position. Dropped findings 26 -> 5.
2. `SynonymConsistency` counted **quoted/cited words as used synonyms** on
meta-scenarios that describe the rule ("a document uses 'check', 'verify',
'confirm'"). Fixed in steve (`6ce4693`): skip words inside `"..."`/`` `...` ``
spans. Dropped findings 5 -> 4.

## What Changes

Two coordinated changes:

1. **steve** (already done): fix `PronounAmbiguity` (`b4e79b5`) and
   `SynonymConsistency` (`6ce4693`). Both benefit all prose, not just scenarios.

2. **veriplan**: apply a **safe subset** of STE rules to scenario step content.
   For each step: strip the `**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**` markers
   and inline code spans, then run **PronounAmbiguity + SentenceLength**.
   Keep PassiveVoice, OneInstructionPerSentence, and SynonymConsistency out
   (proven to false-positive on legitimate scenario structure or meta-scenarios).

**Validated on the real corpus (196 steps, after both steve fixes):**
- Clear false positives: **0** (the "does not flag it" and meta-scenario
  synonym cases are resolved).
- True positive: **1** (a genuinely long step caught by SentenceLength).
- Residual borderline/defensible: **3** — one residual pronoun false positive
  (adjective-noun: "a legitimate technical noun ... its" where "legitimate"
  is counted as a noun), and two defensible cases (compound-noun "modal-ladder",
  and "Bazel ... its dependencies" which is mildly ambiguous).

## Capabilities

### New Capabilities
- `scenario-ste-prose`: Apply a safe subset of STE prose rules
  (PronounAmbiguity, SentenceLength) to scenario step content, stripping the
  GIVEN/WHEN/THEN/AND scaffolding and code spans first.

### Modified Capabilities
<!-- none — this is a new capability; no existing spec-level behavior changes -->

## Impact

- **steve** (`../steve`): `PronounAmbiguity` fix (`b4e79b5`) and
  `SynonymConsistency` fix (`6ce4693`); 4 regression tests added in total.
  Requires a steve release/tag before veriplan reverts to the git dependency.
- `src/prose/mod.rs`: extend `check_prose` to also check scenario steps with the
  safe rule subset; add a `rules_for_scenario()` helper (or similar).
- `src/parser/helpers.rs`: `extract_scenarios` already parses steps — reuse it;
  add marker/code-span stripping for the prose pass.
- `src/ir/mod.rs`: no `ConstraintCategory` change; findings flow through the
  existing `ProseFinding` / rephrase-directive pipeline.
- Tests: unit tests for the safe subset (no false positives on legitimate
  scenarios; true positive on vague scenario; PassiveVoice/OneInstruction/
  SynonymConsistency stay out).
- **No behavior change to verdicts** — prose findings remain advisory
  (rephrase directives), never blocking.
- **Dependency note**: veriplan's `Cargo.toml` currently points at local steve
  (`../steve/steve`) for development; revert to the git tag once steve ships the
  fix.

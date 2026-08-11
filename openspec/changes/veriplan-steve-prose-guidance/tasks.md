## 1. steve-side changes (in ../steve, additive — see specs/steve-integration)

- [ ] 1.1 Add steve builder method to accept excluded line ranges for prose-zone scoping (spec R1.1); findings wholly inside an excluded range are suppressed, boundary-spanning findings kept in the included region
- [ ] 1.2 Add steve builder method to configure max sentence length, independent of TextKind defaults (spec R2.1); default unchanged when not set
- [ ] 1.3 Keep both changes additive — no behavior change to existing steve callers; steve's own test suite still passes

## 2. Add steve dependency to veriplan

- [ ] 2.1 Add `steve = { path = "../steve" }` to Cargo.toml
- [ ] 2.2 Confirm steve builds and `Ste::builder()` is accessible from veriplan
- [ ] 2.3 Verify the crate resolves in this workspace (path dependency outside workspace root)

## 3. Prose-zone selection (D2 — load-bearing)

- [ ] 3.1 Add a module (e.g. `src/prose/mod.rs`) that, given a parsed PlanIR, selects prose zones per artifact: requirement body paragraphs for spec.md, task descriptions for tasks.md, body paragraphs for design.md/proposal.md
- [ ] 3.2 Exclude scenario scaffolding: `**GIVEN**/**WHEN**/**THEN**`/`**AND**` list items and `#### Scenario:` blocks from steve input
- [ ] 3.3 Exclude inline code spans (`` `...` ``) and predicate keywords (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE) from steve input
- [ ] 3.4 Use steve exclusion-range scoping (task 1.1) to preserve line/column provenance

## 4. Curated rule set + strictness mapping (D1, D3)

- [ ] 4.1 Build a `Ste` per artifact with all rules disabled except the curated set: PassiveVoice, PronounAmbiguity, Hedging, OneInstructionPerSentence, SynonymConsistency, SentenceLength
- [ ] 4.2 Apply per-artifact rule subsets: full set for spec.md; OneInstructionPerSentence + Hedging for tasks.md; PassiveVoice + PronounAmbiguity + Hedging for design.md/proposal.md
- [ ] 4.3 Map steve finding severity from StrictnessProfile: Strict → PassiveVoice/OneInstruction hard, rest soft; Moderate → all soft; Lax → all info

## 5. Report integration + grounding correlation (D4, D5)

- [ ] 5.1 Emit prose findings as rephrase directives in the convertibility report
- [ ] 5.2 Ensure prose findings NEVER contribute a blocker (plan stays ConvertibleWithWarnings at worst)
- [ ] 5.3 Correlate steve style findings (passive/pronoun/hedging) with grounding outcomes per requirement; when both present on the same requirement, emit ONE combined directive ("...passive AND ungrounded — name the agent as a task ID...")
- [ ] 5.4 Run the correlation in all strictness modes including Lax (where ungrounded is info-only); combined directive still emitted at info severity
- [ ] 5.5 Surface prose findings through LSP diagnostics (existing pipeline)

## 6. Testing

- [ ] 6.1 Unit test: curated rules disable dictionary/noun-cluster/topic-sentence (no false findings on "shall", "create", "**GIVEN**")
- [ ] 6.2 Unit test: scenario `**THEN**` step "the plan SHALL be marked VALID" produces NO passive finding (D2 exclusion works)
- [ ] 6.3 Unit test: per-artifact rule subsets apply (tasks.md gets one-instruction only, not passive)
- [ ] 6.4 Unit test: strictness mapping (Strict/Moderate/Lax severities)
- [ ] 6.5 Unit test: combined rephrase directive when passive + ungrounded on same requirement
- [ ] 6.6 Unit test: no combined directive when requirement is active + grounded
- [ ] 6.7 Unit test (steve): exclusion range suppresses in-range findings, keeps boundary-spanning finding in included region
- [ ] 6.8 Unit test (steve): configured max sentence length is honored, default unchanged
- [ ] 6.9 Integration: run `veriplan check` on a sample change; prose findings appear as warnings/info but never block

## 7. Dogfooding against veriplan's and steve's own specs

- [ ] 7.1 Run the feature over `openspec/specs/*` and archived `openspec/changes/archive/*` (46 specs, 13 tasks, 13 designs, 13 proposals)
- [ ] 7.2 Confirm noise is low (~4.5 findings/file) and all findings come from curated rules
- [ ] 7.3 Confirm scenario `**THEN**`/`**WHEN**` steps (e.g. "SHALL be marked VALID" in model-check/spec.md) are NOT flagged
- [ ] 7.4 Confirm real passive+ungrounded requirements (e.g. "A .md file path SHALL be resolved" in input-resolution/spec.md) ARE flagged with a combined directive
- [ ] 7.5 Confirm the same curated config runs cleanly over steve's own `openspec/` tree (29 specs, 12 tasks) via `steve/examples/openspec_prose.rs`

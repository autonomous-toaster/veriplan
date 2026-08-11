## 1. Add steve dependency

- [ ] 1.1 Add `steve = { path = "../steve" }` to Cargo.toml
- [ ] 1.2 Confirm steve builds and `Ste::builder()` is accessible from veriplan
- [ ] 1.3 Verify the crate resolves in this workspace (path dependency outside workspace root)

## 2. Prose-zone selection (D2 — load-bearing)

- [ ] 2.1 Add a module (e.g. `src/prose/mod.rs`) that, given a parsed PlanIR, selects prose zones per artifact: requirement body paragraphs for spec.md, task descriptions for tasks.md, body paragraphs for design.md/proposal.md
- [ ] 2.2 Exclude scenario scaffolding: `**GIVEN**/**WHEN**/**THEN**`/`**AND**` list items and `#### Scenario:` blocks from steve input
- [ ] 2.3 Exclude inline code spans (`` `...` ``) and predicate keywords (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE) from steve input
- [ ] 2.4 Preserve line/column provenance for findings (prefer steve exclusion-range scoping over substring slicing; see D6)

## 3. Curated rule set + strictness mapping (D1, D3)

- [ ] 3.1 Build a `Ste` per artifact with all rules disabled except the curated set: PassiveVoice, PronounAmbiguity, Hedging, OneInstructionPerSentence, SynonymConsistency, SentenceLength
- [ ] 3.2 Apply per-artifact rule subsets: full set for spec.md; OneInstructionPerSentence + Hedging for tasks.md; PassiveVoice + PronounAmbiguity + Hedging for design.md/proposal.md
- [ ] 3.3 Map steve finding severity from StrictnessProfile: Strict → PassiveVoice/OneInstruction hard, rest soft; Moderate → all soft; Lax → all info

## 4. Report integration + grounding correlation (D4, D5)

- [ ] 4.1 Emit prose findings as rephrase directives in the convertibility report
- [ ] 4.2 Ensure prose findings NEVER contribute a blocker (plan stays ConvertibleWithWarnings at worst)
- [ ] 4.3 Correlate steve style findings (passive/pronoun/hedging) with grounding outcomes per requirement; when both present on the same requirement, emit ONE combined directive ("...passive AND ungrounded — name the agent as a task ID...")
- [ ] 4.4 Run the correlation in all strictness modes including Lax (where ungrounded is info-only); combined directive still emitted at info severity
- [ ] 4.5 Surface prose findings through LSP diagnostics (existing pipeline)

## 5. Testing

- [ ] 5.1 Unit test: curated rules disable dictionary/noun-cluster/topic-sentence (no false findings on "shall", "create", "**GIVEN**")
- [ ] 5.2 Unit test: scenario `**THEN**` step "the plan SHALL be marked VALID" produces NO passive finding (D2 exclusion works)
- [ ] 5.3 Unit test: per-artifact rule subsets apply (tasks.md gets one-instruction only, not passive)
- [ ] 5.4 Unit test: strictness mapping (Strict/Moderate/Lax severities)
- [ ] 5.5 Unit test: combined rephrase directive when passive + ungrounded on same requirement
- [ ] 5.6 Unit test: no combined directive when requirement is active + grounded
- [ ] 5.7 Integration: run `veriplan check` on a sample change; prose findings appear as warnings/info but never block

## 6. Dogfooding against veriplan's own specs

- [ ] 6.1 Run the feature over `openspec/specs/*` and archived `openspec/changes/archive/*` (46 specs, 13 tasks, 13 designs, 13 proposals)
- [ ] 6.2 Confirm noise is low (~4.5 findings/file) and all findings come from curated rules
- [ ] 6.3 Confirm scenario `**THEN**` steps (e.g. "SHALL be marked VALID" in model-check/spec.md) are NOT flagged
- [ ] 6.4 Confirm real passive+ungrounded requirements (e.g. "A .md file path SHALL be resolved" in input-resolution/spec.md) ARE flagged with a combined directive

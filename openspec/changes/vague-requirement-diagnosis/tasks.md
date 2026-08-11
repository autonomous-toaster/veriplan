## 1. Vague-requirement diagnosis helpers (in translator)

- [ ] 1.1 Add `diagnose_bare_capability(statement, task_refs)` — returns BareCapability when a non-formalizable requirement references a task and contains no vague word (spec R1.1)
- [ ] 1.2 Add `diagnose_vague_action(statement, task_refs)` — returns VagueAction when a non-formalizable requirement references a task and contains a vague adverb (spec R1.2)
- [ ] 1.3 Add `diagnose_vague_quality(statement)` — returns VagueQuality when a non-formalizable requirement has no task reference and contains a vague adjective (spec R1.3)

## 2. Safety boundary + classification integration

- [ ] 2.1 Add a `VagueDiagnosis` enum and a `diagnose_vague(statement, task_refs) -> Option<VagueDiagnosis>` helper invoked only when `classify()` returns `NonFormalizable`; the classification priority (temporal → informational → NonFormalizable) is unchanged so temporal requirements are never diagnosed (spec R2.2)
- [ ] 2.2 Add the small curated vague-word list (adverbs, adjectives, comparatives), hardcoded like the temporal-keyword list; document that legitimate safety *constraints* classify as temporal and are never diagnosed

## 3. Checker integration + targeted fixes

- [ ] 3.1 In `check_classifiability`, when `classify()` returns `NonFormalizable`, call `diagnose_vague` and use the diagnosis to populate the blocker's `detail` and `fix` with the targeted, pedagogical message (bare-capability / vague-action / vague-quality)
- [ ] 3.2 Keep the existing generic blocker ("does not match any temporal category") as the fallback when the diagnosis returns None (undiagnosed — no task ref, no vague word)

## 4. Testing

- [ ] 4.1 Unit test: "T1.1 SHALL be executed." → BareCapability diagnosis + targeted fix
- [ ] 4.2 Unit test: "T1.1 SHALL be done quickly." → VagueAction diagnosis + targeted fix
- [ ] 4.3 Unit test: "The system SHALL be robust." → VagueQuality diagnosis + targeted fix
- [ ] 4.4 Unit test: "T1.1 SHALL be done quickly BEFORE T1.2 SHALL start." → SequentialOrder, NOT diagnosed as vague (safety boundary)
- [ ] 4.5 Unit test: "The migration SHALL happen." (no task ref, no vague word) → generic blocker fallback
- [ ] 4.6 Verify all verdicts unchanged — vague requirements still block; no requirement is reclassified to informational/non-blocking

## 5. Dogfood + verify

- [ ] 5.1 Run `veriplan check` on this change's own specs to confirm the new spec delta passes (all requirements formalizable, temporal grammar correct)
- [ ] 5.2 Run the full veriplan test suite (existing 254+ tests still pass; only new diagnosis behavior added)
- [ ] 5.3 Confirm the archived `veriplan-plan-verifier` change still shows its intended blocker path (generic message) for genuinely undiagnosed requirements, and that any vague requirements it contains now show the targeted fix

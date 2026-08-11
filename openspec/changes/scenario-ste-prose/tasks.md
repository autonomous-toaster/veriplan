## 1. Scenario STE checking (in prose module)

- [ ] 1.1 Add a `rules_for_scenario()` helper returning the safe subset (PronounAmbiguity, SynonymConsistency, SentenceLength) — excludes PassiveVoice and OneInstructionPerSentence (spec R1.3)
- [ ] 1.2 Add a step-content stripper that removes `**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**` markers and inline code spans before checking (spec R1.2)
- [ ] 1.3 Extend `check_prose` to parse scenario steps via `extract_scenarios` and check each step's stripped content with the safe subset (spec R1.1, R3.1)

## 2. Report integration

- [ ] 2.1 Emit scenario prose findings as rephrase directives through the existing `ProseFinding` pipeline; ensure they NEVER contribute a blocker (spec R2.1)

## 3. Parser reuse

- [ ] 3.1 Confirm `extract_scenarios` returns GIVEN/WHEN/THEN/AND steps usable by the prose pass; add any needed accessor (spec R3.1)

## 4. Testing

- [ ] 4.1 Unit test: safe subset on a legitimate scenario ("**THEN** the plan SHALL be marked VALID") produces NO PassiveVoice/OneInstruction finding (spec R1.2, R1.3)
- [ ] 4.2 Unit test: safe subset on a genuinely vague scenario ("it SHALL respond appropriately") produces a PronounAmbiguity finding (spec R1.1)
- [ ] 4.3 Unit test: scaffolding stripping removes markers/code spans before checking (spec R1.2)
- [ ] 4.4 Unit test: scenario prose findings are advisory (rephrase directive, never blocking) (spec R2.1)
- [ ] 4.5 Verify all verdicts unchanged — no requirement is newly blocked by scenario prose

## 5. Dogfood + verify

- [ ] 5.1 Run `veriplan check` on this change's own specs to confirm the new spec delta passes (all requirements formalizable, temporal grammar correct)
- [ ] 5.2 Run the full veriplan test suite (existing tests still pass; only new scenario-prose behavior added)
- [ ] 5.3 Confirm existing specs with scenarios produce no new false-positive prose findings

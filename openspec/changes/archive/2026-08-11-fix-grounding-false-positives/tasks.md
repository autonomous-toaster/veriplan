## 1. Skip MAY Requirements in Grounding

- [x] 1.1 Add MAY strength check to `check_grounding()` in `src/grounding/mod.rs`: skip requirements with `Rfc2119Strength::May`
- [x] 1.2 Add info-level CheckItem when MAY requirements are skipped (for transparency)
- [x] 1.3 Ensure skipped MAY requirements don't appear in outcomes list (no PatternUngrounded population)

## 2. Improve Error Messages

- [x] 2.1 In the Ungroundable branch, check if any constant name from the Signature appears in the requirement text
- [x] 2.2 If task IDs are present but no predicate keyword matched, use message: "no matching predicate keyword found (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE)"
- [x] 2.3 If no task IDs are present, keep existing message: "no matching task or predicate found"
- [x] 2.4 Update rephrase directive to match the error message variant

## 3. Testing

- [x] 3.1 Unit test: MAY requirement is skipped by grounding check
- [x] 3.2 Unit test: MAY requirement produces no blockers or warnings
- [x] 3.3 Unit test: Requirement with task ID but no predicate keyword produces correct error message
- [x] 3.4 Unit test: Requirement with no task ID and no predicate keyword produces original error message

## 4. Dogfooding

- [x] 4.1 Run `veriplan check veriplan-plan-verifier --phase convertibility` and verify the deprecated BFS fallback requirement no longer produces a grounding blocker
- [x] 4.2 Run `veriplan check veriplan-plan-verifier --phase convertibility --format json` and verify JSON output

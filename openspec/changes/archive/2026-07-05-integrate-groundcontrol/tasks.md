## 1. Dependency Setup

- [x] 1.1 Add groundcontrol git dependency to Cargo.toml: `groundcontrol = { git = "https://github.com/autonomous-toaster/groundcontrol", rev = "<commit-sha>" }`
- [x] 1.2 Create `src/grounding/` module directory with `mod.rs`
- [x] 1.3 Add `pub mod grounding;` to `src/lib.rs`

## 2. Signature Builder

- [x] 2.1 Implement `Signature::from_planir(plan: &PlanIR) -> Signature` that maps each PlanIR Task to a ConstantDef (name = "T{id}", aliases from description)
- [x] 2.2 Include all 6 predicate definitions (BEFORE, AFTER, CONCURRENTLY, IF_THEN, ALWAYS, AT_MOST_ONE) with correct argument slots
- [x] 2.3 Include type definitions (task_id, phase_name)
- [x] 2.4 Sort constants by name for deterministic output
- [x] 2.5 Handle empty PlanIR gracefully (zero constants, predicates still present)

## 3. Grounding Module

- [x] 3.1 Implement `check_grounding(plan: &PlanIR, strictness: &StrictnessProfile) -> (Vec<CheckItem>, Vec<CheckItem>, Vec<CheckItem>)` that returns (blockers, warnings, info)
- [x] 3.2 For each requirement, build Signature from PlanIR and call `RuleGrounder::ground()` on the requirement's SHALL statement
- [x] 3.3 Map `Ungroundable` status to blocker CheckItem with rephrase directive suggesting explicit task IDs
- [x] 3.4 Map `Ambiguous` status to blocker CheckItem by default, with close match suggestions
- [x] 3.5 Apply strictness profile: Moderate downgrades ambiguous to warning, Lax downgrades both ambiguous and ungroundable to warning
- [x] 3.6 Map `Grounded` status to no CheckItem (pass silently)
- [x] 3.7 Populate `PatternUngrounded` ConstraintCategory on requirements that fail grounding

## 4. Pipeline Integration

- [x] 4.1 Add grounding check call to `check_convertibility()` in `src/checker/convertibility.rs` between T4.2 (requirement references) and T4.4 (temporal classification)
- [x] 4.2 Pass strictness profile through to the grounding check
- [x] 4.3 Skip grounding check for requirements that already failed T4.2 (no point grounding if task IDs don't exist)
- [x] 4.4 Skip grounding check in stdin/single-file mode (no Signature to build against)

## 5. Report Integration

- [x] 5.1 Add grounding blockers to the convertibility report's blockers list
- [x] 5.2 Add grounding warnings to the convertibility report's warnings list
- [x] 5.3 Add grounding rephrase directives to the report's rephrase_directives list
- [x] 5.4 Ensure grounding results appear in JSON output format

## 6. Bootstrap Integration

- [x] 6.1 Add grounding rules to `veriplan bootstrap` config generation: explicit task IDs, parenthetical syntax, BEFORE requires two IDs, ALWAYS requires one ID
- [x] 6.2 Add grounding context to `veriplan bootstrap`: "Every task reference in a spec MUST map to a real task ID. Use explicit N.M IDs — not descriptions like 'the migration step'."

## 7. Testing

- [x] 7.1 Unit test: Signature from PlanIR with multiple tasks and phases
- [x] 7.2 Unit test: Signature from empty PlanIR
- [x] 7.3 Unit test: Grounding passes for explicit task IDs
- [x] 7.4 Unit test: Grounding passes for NL aliases
- [x] 7.5 Unit test: Grounding fails for vague NL with no task reference
- [x] 7.6 Unit test: Ambiguous grounding produces blocker by default
- [x] 7.7 Unit test: Ambiguous grounding downgraded to warning with Moderate strictness
- [x] 7.8 Unit test: Grounding skipped for requirements that failed T4.2
- [x] 7.9 Integration test: Full pipeline with grounding on veriplan's own specs

## 8. Dogfooding

- [x] 8.1 Run `veriplan check integrate-groundcontrol --phase convertibility` and verify grounding results appear
- [x] 8.2 Run `veriplan bootstrap` and verify grounding rules are in config.yaml
- [x] 8.3 Verify JSON output includes grounding results

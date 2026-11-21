## Context

See proposal.md for motivation. Three code paths combine to make a verdict that reads stronger than the work performed.

`classify()` (`src/translator/mod.rs`) tests `exclusive`, `conditional`, `concurrent`, `fixed_time`, `global`, then `sequential`. `is_time_ref` is `contains("min") || contains("hour") || contains("sec") || contains(":00") || contains("am") || contains("pm") || any_ascii_digit`. Since a model-checked requirement always cites a task id, `is_time_ref` is effectively true, so `is_fixed_time`'s `before`/`after` clause shadows `SequentialOrder` always.

`generate_ltl` (`src/translator/ltl.rs`) has one arm that admits no encoding:

```rust
FixedTime | Global => {
    // Global invariants and fixed-time constraints without reliable durations
    // Just note the constraint exists — evaluated as always-true placeholder
    Some(LtlFormula::Always(LtlCondition::Atom("true".into())))
}
```

`translate_all` produces one constraint per requirement, whatever the body contains, and `spin.rs`/`spin_rs.rs` count every constraint with a formula as satisfied or violated. A placeholder is therefore indistinguishable from a proof in the report, in the JSON, and in the exit code.

The engine is chosen by `VERIPLAN_CHECKER` (`src/cmd_check.rs`), defaults to `spin`, and the choice appears in no output field.

Measured on a downstream artifact (62 published requirements plus the archived task list of the change that produced them): `--checker spin` reports 4 violations; `--checker spin-rs` reports 0. On the same corpus, 58 of 62 formulas are `[] ( true )`.

## Goals / Non-Goals

**Goals:**
- A requirement that expresses an ordering reaches `SequentialOrder` and is model-checked.
- A constraint that is not modelled is reported as not modelled and is not counted as satisfied.
- The verdict names the engine and states how many constraints were modelled.
- An engine that cannot refute a known-false fixture SHALL NOT produce a validity verdict.
- Scenario-step findings carry file-absolute coordinates.
- A requirement citing a task that an archived change defines resolves, and the finding names the owning change.

**Non-Goals:**
- Real time semantics. This change removes the false claim that fixed-time constraints are checked; it does not add durations, clocks or a timed model.
- Merged-model checking (published specs union the delta in one state space). That is a separate change, `baseline-conformance`; the argument is in D6.
- Re-validating archived changes. The archive is a grounding source and a baseline, never a check target (D6).
- Repairing the corpus. Plans that used placeholder formulas will report new findings; fixing those plans is downstream work, not part of this change.
- Changing the engine. `spin-rs` soundness is a separate change in its own repository; this change makes an unsound engine visible rather than usable.
- Repairing the published corpus. Placeholder constraints in the 24 published capabilities are reported by the repaired translator; editing those capabilities is downstream work.

## Decisions

### D1 — Sequential order outranks fixed time, and a time reference is a clock

`is_sequential` is tested before `is_fixed_time`, and `is_time_ref` matches a clock or a duration literal (`09:30`, `2 minutes`, `3 hours`) instead of any digit. A body citing task ids that uses BEFORE or AFTER therefore classifies as `SequentialOrder`.

**Why not special-case task ids inside `is_time_ref`?** That keeps a heuristic whose intent is "this sentence talks about time" answering a different question ("does this sentence contain a number"). Any future id form, count or version number reintroduces the bug. A time reference means a time.

**Consequence, accepted:** statements that genuinely express a time window now also classify as `SequentialOrder` unless `is_time_ref` matches, and a body containing both a real window and an ordering is a two-clause body, which D5 reports instead of silently truncating.

**Evidence that the seam was never tested:** the published `rule-translator` capability states, under "Generate LTL for sequential order", that `T1.1 SHALL complete before T1.2` produces `G (active(T1.1) → done(T1.2))`. The direction is inverted: the implementation emits `active_t1_2 -> done_t1_1`, which is correct, and the spec says the opposite. Both halves were specified, neither was compared to the other. Task 1.3 is that comparison.

### D2 — A category without an encoding yields no formula

The `FixedTime | Global` arm returns `None` and the constraint carries the reason. No placeholder is emitted.

**Why not implement fixed-time and global properly?** The generated Promela has no clock and no duration; a real encoding would need a timed model, which is a different tool. Emitting `[] ( true )` and calling the result "satisfied" is the defect; the honest output is "this constraint is not modelled".

**Why not block such a plan outright?** Because the corpus already relies on the concept: `informational-requirements` exists for requirements that are human-review-only by design. An unmodelled constraint joins that mechanism instead of a parallel one: it is declared, reported, excluded from the satisfied count, and it blocks only where the strictness ladder already blocks informational requirements.

### D3 — The count is split, not merged

`satisfied_constraints` SHALL count only `Modelled` constraints that the engine did not refute; `unmodelled_constraints` is reported beside it. `total_constraints` SHALL equal the sum plus violations.

**Why not keep one number?** Because the number is the artifact people quote. A single "Satisfied: 62" is what let a repository believe 62 properties were proved. Two numbers make the claim self-describing.

### D4 — A canary turns an unsound engine into an unverified run

The Promela generator emits a fixture model whose expected verdict is known: two tasks in one concurrent phase with `AT MOST ONE` between them, refutable by any correct engine. Each backend SHALL refute the canary before its result is used for the plan. A backend that reports the canary satisfied SHALL produce `valid: null` with the canary named in `skip_reason` — unverified, not invalid and not valid.

**Why a canary and not a dependency on the engine fix?** Because the two repositories ship independently, and a gate must not silently improve when an unrelated repository changes. The canary makes the contract executable in both directions: the engine repository can test against it, and this repository refuses to certify without it.

**Why `valid: null` and not a hard error?** A hard error would make `--checker spin-rs` unusable while the engine fix is in flight, which removes the choice rather than informing it. `null` already exists in the result type and already means "no verdict"; pre-commit mode already treats `null` as non-blocking.

### D5 — One constraint per body, enforced by a finding

`split_constraint_clauses` already exists in the grounding module and already detects multiple constraint clauses; today it only fires when the grounder reports two predicates. The same split SHALL be applied to every requirement body, and a body with more than one clause SHALL produce a finding naming the clauses that were not modelled.

**Why a finding and not an error?** The published corpus contains multi-clause bodies. Reporting them is the repair list; refusing them is a migration nobody asked for.

### D6 — The archive is a grounding source, never a check target

An archived change is not model-checked as a change: its tasks are complete, its findings are unactionable, and its proofs are historical. The archived task lists ARE used: the grounding signature is built from the active change's tasks plus the tasks of archived changes, so a requirement that modifies a published requirement resolves against the tasks that originally grounded that requirement.

A task id absent from the active change but present in an archived change SHALL produce a finding that names the owning change, not "add explicit task ID references".

**Why not merged-model checking here?** Checking the published specs union the delta needs one state space over two `PlanIR` values; `verify_all` today verifies each plan separately and merges the reports, which is the wrong shape for the question. It also needs the published specs to be free of placeholder formulas first, or the merged run repeats this change's defect on a larger corpus. That work is `baseline-conformance`, and it is blocked by this change landing.

### D7 — Coordinates: the class, not the instance


`parse_one_scenario` records `start_line: i + 1` where `i` indexes the requirement body, not the file. Every step finding therefore reports a body-relative line. The fix computes the body's file offset once, and the sweep task covers every remaining constructor that builds a `SourceLocation` from a snippet.

**Why a sweep?** The prose path already had this bug and was fixed one instance at a time in `2026-09-01`. The step path survived it. A sweep with one test per path is cheaper than the next instance.

## Risks

- **The model does not scale, and that is now visible.** Every task is its own proctype, so the model is a free interleaving of all of them. Measured with the in-process engine after the other fixes: 2 tasks 0.04 s, 4 tasks 0.08 s, 6 tasks 0.71 s, 8 tasks 10 s. Extrapolated, a 26-task plan does not close in 200 000 states or 7 minutes, and this change's own plan therefore reports UNKNOWN with 19 unchecked constraints. Before this change the same plan reported VALID with 19 "satisfied" constraints, which were placeholders. The scaling fix is a change of model shape — execute a sequential phase in one process, spawn proctypes only for `[concurrent]` phases — and it is deferred: it changes what a verdict means, and it needs its own design.

- **Plans that passed will report findings.** This is the intended output: placeholder constraints become `Unmodelled`, and real ordering violations become violations. The change is a correctness change, and the corpus repair is separate.
- **17 of 23 archived changes fail the current gate today.** Measured by re-checking the archive against this binary. The change does not attempt to repair the archive; it documents why the archive is not a check target (D6).
- **`--checker spin-rs` becomes UNVERIFIED until its engine is fixed.** The engine cannot explore a model containing `if … fi` inside `do … od`, which the generator emits in every proctype; on such a model it reports one state, zero transitions and success. The canary converts that into an unverified verdict. The fix is handed off with reproducers; this change must not depend on it landing.
- **Ordering violations may appear in the published corpus once real formulas are emitted.** Expected and desirable; they are the reason for the split count.
- **Classification changes are load-bearing.** Every test that asserts a category for a statement containing a task id and "before" now asserts a different category; the seam test in task 1.3 is the guard.

## Migration Plan

1. Land classification and placeholder removal (phase 1) with the seam test.
2. Land the modelled split and the backend label (phase 2) so the new output is self-describing before any corpus is re-checked.
3. Land the canary (task 2.6) last in its phase, so the conformance failure arrives after the label that explains it.
4. Coordinates and baseline grounding (phases 3 and 4) are independent of 1 and 2 and can land in either order.

## Self-check

- Every requirement body is one sentence with one temporal keyword and two task ids, in execution order, so it survives the parser this change is fixing.
- No requirement in this change relies on `FixedTime`, `Global` or `SequentialOrder` remaining unmodelled.
- The change contains no placeholder: it cannot pass by being the thing it removes. Once phase 1 lands, the constraints in its own specs are emitted as real formulas and are model-checked.


### D8 — The init rule moves the task table out of the spec delta

`veriplan init` writes a rule instructing the author to add a `Task Reference` section before the first `### Requirement:`. `openspec archive` does not carry an extra section into the published spec, and `openspec validate --strict` reports a warning for every such section. Both were reproduced while writing this change:

```
⚠ [WARNING] rule-translator/spec.md: Section "## Task Reference" is not carried into the main
spec by archive. Move it into a requirement, or into proposal.md or design.md.
```

The rule SHALL point at the design artifact. This keeps the provenance a reviewer needs and stops the generated configuration from prescribing an artifact the archive rejects.

**Why not keep the table in the delta?** Because the warning is permanent, the section is dropped at archive, and the author has no way to know which of the two tools is authoritative. A rule that produces a warning on every compliant spec is a rule that trains authors to ignore warnings.

## Task Reference

| T ID | Description |
|------|-------------|
| T1.1 | Test sequential order ahead of fixed time in classify |
| T1.2 | Narrow is_time_ref to a clock or duration literal |
| T1.3 | Add a seam test over classify and generate_ltl |
| T1.4 | Remove the always-true placeholder from the fixed-time and global arms |
| T1.5 | Reject a temporal relation between one task and the same task |
| T1.6 | Report a requirement body whose second constraint clause was not modelled |
| T2.1 | Mark each translated constraint as modelled or unmodelled |
| T2.2 | Exclude an unmodelled constraint from the satisfied count |
| T2.3 | Name the checker backend in the human verdict |
| T2.4 | Add the backend and the modelling split to the JSON output |
| T2.5 | Fail a search that explores no transitions on a non-empty model |
| T2.6 | Add the calibration canary to the generated model set |
| T3.1 | Compute scenario-step sources against the file |
| T3.2 | Sweep every remaining snippet-relative source constructor |
| T3.3 | Add coordinate tests for a step finding and a prose finding |
| T4.1 | Build the grounding signature from archived task lists |
| T4.2 | Report a cited task id owned by an archived change |
| T4.3 | Document the archive policy in the grounding spec |
| T4.4 | Relocate the init task table rule into the design artifact |
| T5.1 | Run openspec validate with --strict |
| T5.2 | Run veriplan check with --strict |
| T5.3 | Record the downstream violations |

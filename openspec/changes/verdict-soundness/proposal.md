## Why

Three defects make `veriplan check` report a verdict stronger than the work performed. A downstream repository ran the gate on 62 constraints, twenty-odd times, and every run said `0 violated`. The artifact was checked again, with the same tool, against the same task list, after the defects below were found: it reports four violations.

1. **Ordering constraints are tautologies.** `classify()` tests `is_fixed_time` before `is_sequential`, and `is_time_ref` matches any ASCII digit. A requirement body always cites a task id, so a body always contains a digit; therefore any "before" or "after" statement classifies as `FixedTime`, which translates to `[] ( true )`. Measured on that repository: 62 `ltl` declarations, 58 of them `[] ( true )`. `SequentialOrder` — the encoding this tool exists to produce — is unreachable in any model-checked plan.
2. **A constraint that was never modelled is counted as satisfied.** `FixedTime` and `Global` return a placeholder formula whose own code comment calls it a placeholder. The report prints "Satisfied: N", the JSON prints `satisfied_constraints`, and nothing distinguishes a proof from a placeholder.
3. **The verdict does not say which engine produced it.** `VERIPLAN_CHECKER` selects the backend. On the same artifact, seconds apart, `--checker spin` reports `✗ INVALID` with four violations and `--checker spin-rs` reports `✓ VALID 62/62`. Neither output names the engine.

4. **A run where every search timed out reports VALID with zero constraints satisfied.** On this change's own plan, `--checker spin` spends 100 s on 17 constraints (a 5 s budget per property), times out on all 17, and reports `valid: true` with `satisfied_constraints: 0`. A timeout is neither satisfied nor violated, and the verdict is derived from the absence of violations alone.

Two smaller defects share a cause with the above: a scenario step reports a body-relative line number as a file coordinate (the same class fixed for prose findings in `2026-09-01-prose-blockers-and-consistent-verdict`, which left the step path behind), and a requirement cites a task id that only an archived change defines, which the grounding check reports as "add explicit task ID references" — the action the author already performed.

The dogfooding gap is the same problem measured on this repository: 17 of 23 archived changes fail the current gate when re-checked today, because the rules tightened (`prose_other` blocks since `09ac08e`) and nothing re-validates the corpus. A tool that cannot report that drift is a tool whose green is not evidence.

## What Changes

- **Sequential order is classified before fixed time**, and a time reference means a clock or duration literal, never a bare digit. A requirement citing task ids and using BEFORE/AFTER reaches `SequentialOrder` and produces a real LTL formula.
- **A category without an encoding produces no formula.** `FixedTime` and `Global` stop returning `[] ( true )`; the constraint is marked `Unmodelled`, reported as a finding with a reason, and SHALL NOT be counted as satisfied.
- **The verdict names the engine and the modelled split.** Human output and JSON both carry the backend and how many constraints were modelled.
- **A search that explores nothing is an error.** A model with at least one proctype that yields zero transitions SHALL fail the run; today it prints a success.
- **A calibration canary makes the two engines agree or fail.** A fixture with a known violation SHALL be reported violated by every backend; a backend that reports the canary satisfied SHALL fail the run.
- **A body whose second constraint clause was not modelled is reported**, instead of the second clause disappearing silently.
- **Scenario-step coordinates are file-absolute**, and every remaining snippet-relative constructor is swept in the same pass.
- **The `veriplan init` rule points at the design artifact.** The generated configuration stops instructing authors to add a `Task Reference` section to the spec delta, which the archive drops and `openspec validate --strict` warns about.
- **The grounding signature includes archived task constants**, so a requirement that cites a task an archived change defines resolves, and the finding names the owning change instead of asking for an id that is already present.

## Capabilities

### New Capabilities

- `backend-conformance`: the producer/consumer contract between the translated LTL and the checking engine — the atom vocabulary, the canary fixture, and the rule that an engine which cannot refute the canary is an error rather than a pass.

### Modified Capabilities

- `rule-translator`: classification precedence changes; `is_time_ref` narrows to clock and duration literals; a category without an encoding yields no formula; identical task arguments are rejected; a body with more than one constraint clause is reported.
- `model-check`: a constraint is `Modelled` or `Unmodelled`; an `Unmodelled` constraint SHALL NOT be counted satisfied; a zero-transition search is an error.
- `output-contract`: the verdict and the JSON carry the backend and the modelled split; scenario-step findings carry file-absolute coordinates.
- `plan-parser`: a scenario step's `SourceLocation` SHALL be file-absolute, like every other element.
- `grounding-check`: the signature is built from the change's tasks and the archived task lists; a task id that only an archived change defines is reported with the owning change.

## Impact

- `src/translator/mod.rs` — `classify`, `is_time_ref`, `generate_ltl` call sites, `diagnose_vague` boundary test.
- `src/translator/ltl.rs` — the `FixedTime | Global` arm; the `[] ( true )` placeholder is removed.
- `src/ir/mod.rs` — a modelling status on the constraint; a finding kind for an unmodelled constraint and for an unmodelled clause.
- `src/checker/mod.rs`, `src/checker/spin.rs`, `src/checker/spin_rs.rs` — the modelled split, the backend label, the zero-transition guard.
- `src/checker/promela.rs` — the canary fixture.
- `src/parser/helpers.rs`, `src/parser/mod.rs` — scenario-step coordinates.
- `src/grounding/mod.rs` — the signature from change plus archive; the foreign-task diagnosis.
- `src/annotator/mod.rs`, `src/cli.rs` — verdict and JSON fields.
- `src/cmd_init.rs` — the generated `specs` rule names the design artifact for the task reference table.
- Behavioural: plans that passed on placeholder formulas now report `Unmodelled` findings, and plans with real ordering violations now report violations. This is the intended output of the change and the reason for the tasks below.
- Cross-repository: the engine side of the contract (`spin-rs`) is a separate change, handed off with reproducers. This change SHALL NOT depend on that fix landing: the canary turns an unfixed engine into a failed run, not a silent pass.

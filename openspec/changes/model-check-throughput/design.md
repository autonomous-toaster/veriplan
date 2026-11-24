# Design

## Task Reference

| T ID | Description |
| --- | --- |
| T1.1 | Add the atom walk and the per-task failure-flag predicate |
| T1.2 | Emit the failure flag and branch only for the named tasks |
| T1.3 | Pin the state count on a closing probe and reject a refuting one |
| T2.1 | Give a stopped search its own cause, allowance and time used |
| T2.2 | Carry and print the states a claim used and was allowed |
| T2.3 | Report the share given and refuse a budget that cannot be honoured |
| T3.1 | Count an undecided claim outside the match tally |
| T3.2 | Name the stopped backend, its claim and its cause |
| T4.1 | Pin the in-process engine's state count and elapsed time |
| T4.2 | Fail the gate on the exponential plan shape |
| T4.3 | Measure the liveness product and hold it to a recorded ceiling |
| T4.4 | Measure a claim blind to a flag some other claim names |
| T5.1 | Run the strict specification gate and the file-length gate |
| T5.2 | Run the plan check with both backends |
| T5.3 | Re-measure the corpus scale and record the gap left open |

## Context

See `proposal.md` — Why, for the measurements.

Three facts about the current code shape the approach.

**The generator's decision is global, the emission is per task.** `src/checker/promela.rs` computes one boolean:

    let needs_failures = constraints.iter().any(|c| c.ltl_string().contains("failed_"));

and then emits the block inside the per-task loop. The predicate is therefore about the whole plan while the cost is paid per task.

**The formula is already a tree.** `crate::ir::ltl::LtlFormula` is `Always(LtlCondition)`, and `LtlCondition` has `Atom`, `Not`, `And`, `Or`, `Implies`, `Iff`, `Eventually`. A constraint's failure atoms are the `Atom` leaves matching `fail_var(id)`. No string matching is needed. `ltl_string()` exists only as a rendering.

**A stopped claim already lands in the report; its cause does not.** `ConstraintSummary` carries `unchecked_reason: Option<String>` and nothing else about the stop. The in-process engine reports `Unverified { kind, reason, states_used, budget }` per claim and `src/checker/spin_rs.rs` maps `kind` and `reason` into that string, dropping `states_used` and `budget`. The external backend has no kind at all: `PanOutcome` is `Passed | Violated | Unchecked | Empty`, and `spin.rs` hardcodes `"state_budget: search timed out"` for every `Unchecked`.

**Comparison already excludes an undecided claim but does not say so.** `compare_one` prints `unverified` and `—` for a claim the peer did not decide, and leaves it out of the tally. The summary line then prints `total - mismatches` of `total`, where `total` counts only compared claims, so four never-compared claims read as full agreement.

## Goals / Non-Goals

**Goals:**

- Remove the generator's per-task branch cost without changing what any constraint means.
- Make every stop in the report name its cause and its cost, and stop one tag standing for two facts.
- Make a comparison that compared nothing say so.
- Catch the next amplifier with an assertion, on the engine as well as on the model.

**Non-Goals (design level):**

- Bounding the in-process engine's liveness path. That is `war2` in the spin-rs tracker; nothing in this repository can substitute for it.
- Making the in-process engine faster per state (`a6fr`), or adding claim-relative state projection (`64n7`).
- Changing what a verdict means for a constraint between two sequential phases. That remains a structural guarantee rather than a checked one, as `q3w3` recorded.
- Replacing the failure block with a separate claim over a deterministic model. That is a semantics change and is listed as an open question.

## Decisions

### D1. The failure-block predicate is a set of task ids, computed by walking the formula

Add a walk that yields every `Atom` name in an `LtlFormula`, then intersect the result with the set of failure flag names the model defines:

    fn tasks_naming_a_failure_flag(constraints, tasks) -> BTreeSet<TaskId>

The per-task loop emits its block only when its id is in that set.

*Alternatives rejected.* Keeping `contains("failed_")` per task still fails on `failed_t1_1` inside `failed_t1_10` — the same prefix defect the translator was fixed for, which would silently give task 1.1 a block it does not need and, worse, let a claim about 1.10 look like it names 1.1. Matching `fail_var(id)` exactly against the *rendered* string is better but still ties a structural decision to a rendering that exists for SPIN's benefit.

*Consequence to keep in mind.* A failure flag exists in the model only when a claim reads it. Anything that later wants to reason about failure without a claim — a report that lists which tasks can fail, for instance — must compute it from the plan, not from the generated model.

### D2. The walker lives with the formula type

`crate::ir::ltl` gains `LtlFormula::atoms()` (and `LtlCondition::atoms()`), returning atom names in declaration order. The checker composes it; it does not reimplement it.

*Rationale.* The translator already owns the tree and the renderer sits next to it. A walker in the checker would be a second traversal of the same type, and the next person to add an `LtlCondition` variant would update one of them.

### D3. `PanOutcome::Unchecked` carries its cause

`Unchecked` becomes `Unchecked(PanStop)`, with

    enum PanStop { WallClock { allowed: Duration }, NotCompleted }

`spin.rs` renders `unchecked_reason` from it: `wallclock: search stopped at the {N}s budget` or `not_completed: search did not complete`. `state_budget` is then reserved for an engine that actually consulted a budget — which, for the external backend, is never, because `pan` has none.

*Alternatives rejected.* A fourth `PanOutcome` variant duplicates the bookkeeping every caller already does for `Unchecked`; a plain `String` in the variant loses the ability to assert on the cause in a test.

*Trade-off accepted.* The reason string is user-visible. `output-contract` requires an unchecked count distinct from the other counts and the backend name; it does not pin the reason text, so this stays within the contract while changing what a reader sees.

### D4. `ConstraintSummary` carries the per-claim numbers, machine-readably

Two optional fields join `unchecked_reason`:

    pub states_used: Option<usize>,
    pub states_allowed: Option<usize>,

populated from `Unverified::states_used` / `Unverified::budget` in `spin_rs.rs`, and left `None` for the external backend, which reports no such pair.

*Rationale.* `unchecked_reason` is prose for a human. A caller deciding whether to raise the budget needs the ratio, and `VerificationResult` is already serialised, so the numbers cost one struct change and no new surface.

*Alternative rejected.* Encoding them into the reason string. That is the defect being fixed: one field carrying two facts.

### D5. The budget reported is the budget the engine used

The in-process engine divides its budget as `max(max_states / claims, 1)` and never redistributes
or clamps the run total, so a caller asking for two states on a three-claim model gets one state per
claim and three explored, and a caller asking for zero gets the same. Veriplan's own
`default_state_budget()` reads `VERIPLAN_MAX_STATES` and falls back to 200 000 when the value is
unparsable, so an explicit zero and a typo both become a positive budget with no word to the caller.

*Decision.* Report the per-claim share the engine was given, and refuse a zero or unreadable budget
rather than substituting one. Do not compare `states_explored` summed against `max_states`: the
per-claim floor can push the total over the request, and a caller reading that as a breach would be
reading the floor, not a fault. Per-claim entries are the comparison that holds.

*Why the refusal matters more than the arithmetic.* A budget of zero is the one value a caller sets
meaning "do not run", and silently turning it into a positive budget is the same class of defect as
a fuse that reports itself under another engine's tag: the caller is told the run was bounded when it
was not.

### D6. A comparison states what it did not compare

`compare_rows` returns a third count, `not_compared`, and the summary line prints it. When a backend stopped on a budget, `compare_rows` names that backend, the first claim it stopped on and the cause, using the fields added in D3 and D4.

*Rationale.* The current line `N of M constraints match` reads as agreement when `M` silently excludes every claim one side never decided. Counting the exclusions is cheaper to get right than changing the denominator's meaning.

*Alternative rejected.* Folding not-compared claims into mismatches. That would report a disagreement where the engines were never asked the same question.

### D7. The engine gate asserts the state count hard and the wall clock soft

The gate lives in `src/checker/engine_gate.rs`, builds its Promela as source rather than through the
generator, and runs `spin_rs::verify` in process over two fixtures:

- a safety chain with one failure branch, asserting the exact state count and an elapsed time against
  a generous multiple of a recorded baseline;
- a liveness chain, asserting the exact count with the invisible bit and without it, and that the
  first is twice the second.

It is a separate file with a separate name because the model's state-space test is deliberately
measured with `pan`, so an engine regression cannot fail a test about the model.

*Why exact counts and not a ceiling.* The first draft asserted "no worse than 4 000" against a
measured 2 552. The spin-rs session pointed out that this catches only a regression larger than 1.6x,
and passes whether or not the engine ever projects a liveness claim — a threshold wide enough to be
comfortable is a threshold that cannot fail. The table records the exact number, so a change in
either direction fails and the message says which direction means what: a fall is the engine
projecting where it did not, and the table needs lowering; a rise is a regression.

*Why the time assertion stays a multiple.* The count is deterministic and the wall clock is not. The
time is there for the order of magnitude, and the count carries the rest.

*Why the detector is tested on recorded curves.* The gate's non-linearity detector is fed the
recorded tables, not a live model. Fed a live exponential fixture, it would stop testing the detector
the moment the engine fixed the fixture: the assertion would go red because the model became linear,
which looks exactly like a regression in the detector. The live numbers live in the tables, where a
change is the signal and the message says which direction means what.

*Alternative rejected.* A single combined assertion. The two numbers fail for different reasons and a combined message would not say which.

*Trade-off accepted.* A wall-clock assertion is the flakiest thing in a suite. The multiple has to be wide enough to survive a shared runner, which means it will not catch a 2x regression — it is there for the order of magnitude, and the state count is there for the rest.

### D8. Two probes, because a refuting claim hides the gap

A safety claim that is false near the root stops both engines early. Measured on the same chain,
claim `[] ( active_t1 -> done_tN )` against `[] ( active_tN -> done_t1 )`:

| N | refuting: pan | refuting: spin-rs | closing: pan | closing: spin-rs |
|---|---|---|---|---|
| 6 | 3 states, 1 error | 694 states | 37 | 694 |
| 10 | 3 states, 1 error | 11 254 states | 61 | 11 254 |

So a comparison that only checks the verdict passes the refuting probe while the engines differ by
three orders of magnitude, and a comparison that only checks the state count is measuring an engine
that chose to keep going. The state-count fixture must therefore carry a claim that holds in every
reachable state, and a refuting fixture is a separate measurement about refutation depth.

*Rationale.* The counting property and the stopping property are different properties. Pinning only
one of them lets the other hide. T1.3 uses the closing probe and pins the distinction, so a later reader cannot quietly swap the two.

*Note.* spin-rs reports the same count on both probes at every N, because it explores the space
whatever the verdicts. That is worth knowing when reading any comparison: an equal count on a
refuting model is not evidence that the closing case is also equal — it is, here, but only by
accident of the engine never stopping early.

### D9. A requirement this change does not satisfy is recorded, not weakened

`model-check / An unchecked constraint is reported` asserts: *"The budget SHALL apply to every backend, including the in-process one: no backend SHALL run a search without a bound."* That assertion is false today for liveness claims, and this change does not make it true. Requirement text is left exactly as it is. `tasks.md` records the gap and the engine issue that closes it.

*Rationale.* Weakening a requirement to match a known defect is how the defect becomes permanent. The requirement is a statement about the system we intend to have.

## Risks / Trade-offs

- **Under-emitting a failure block hides a conditional constraint's refutation** → the per-task set is computed from every constraint's formula, and a test keeps a plan with a conditional constraint refuting; D1's test asserts both the named and the unnamed side of the same plan.
- **Removing failure flags changes fixture state counts** → re-measure every fixture that asserts a count, in the same change (T5.3), rather than adjusting the expected numbers blind.
- **A new `LtlCondition` variant is added and the walker misses it** → the walker is exhaustive by match with no catch-all arm, so a new variant fails to compile.
- **The wall-clock assertion flakes on a shared runner** → wide multiple, state count carries the hard assertion, and the test skips when the engine refuses the model.
- **Anything parsing `unchecked_reason` for `state_budget` breaks** → `state_budget` keeps its meaning and only stops being used where no budget was consulted; the new `wallclock` and `not_completed` tags are additive.
- **A refuting claim is mistaken for a state-count probe** → D8: the count fixture carries a claim that holds in every reachable state, and T1.3 fails if the two are swapped. This risk was found by the spin-rs session reviewing a probe of mine that refuted at three states on `pan` while their engine paid 11 254.
- **The per-claim relay reports nothing for a liveness claim, because the engine never returns** → the relay reports what the engine reported and invents nothing; T5.4 records this as the gap rather than filling it with a budget that was never consulted.

## Migration Plan

No data migration and no configuration change. The sequence is: generator predicate first (it shrinks the state space the later measurements are taken on), then the outcome and summary types, then the output, then the comparison, then the gate, then the re-measurement. Rollback is a revert of the change; the only behaviour that would revert with it is a smaller model.

## Settled by the spin-rs session

Recorded so the two changes can be reviewed against each other. Their change is
`veriplan-perf-parity` in the spin-rs repository, with the engine work in phases 2 to 4 and the
instrumentation in phase 1.

- The state projection is **claim-relative**, not a global dead-variable pass. Their
  `visible_fields()` is already per formula, so per claim is the cheaper construction as well as
  the one this generator needs: two claims with different visible sets must produce different
  counts.
- `check_violation` does **not** need to change for the projection to be sound. It reads the
  assertion marker and returns before deadlock detection, and deadlock detection only runs when
  the model carries no claim. The always-visible set is therefore the assertion marker, plus the
  running-process fields for a claim-free run. They assert verdict preservation as a requirement of
  their own, so this change can promise verdict preservation downstream.
- Throughput is pinned by fixture and method, not only by a number: at least 1e5 states/s on N=8 in
  a release build, median of three runs. Bitstate and hash-compact storage are excluded as the route,
  because trading completeness for speed cannot underwrite a state-count or verdict claim.
- The per-claim budget rule is `max(max_states / claims, 1)`, with no redistribution and no clamp
  on the run total, measured on their side: `--max-states 2`, `1` and `0` all give one state per
  claim and three explored on a three-claim model. No claim is starved; the defect is the opposite
  of what I guessed, and it is why D5 reports the share rather than comparing a sum, and why a zero
  budget has to be refused here instead of being floored on their side. They have specified the rule
  in their own output, so veriplan no longer has to read their source to predict it.
- Their liveness budget also has to stop returning `None` for both "closed" and "did not finish".
  Budgeting without separating those would let a partial search read as clean, which is worse than
  the hang. That is a stronger fix than the one this proposal asked for, and it is the reason
  design D9 records the gap instead of papering over it.

## Measurements taken after the change

Every number below comes from a run made after the generator fix, through `veriplan` at the pinned
spin-rs tag v0.5.35 (b31e386). Times are wall clock on one machine and are recorded as an order of
magnitude, not as a benchmark.

**Downstream plan, both backends.** Shape: the `chevron-credentials` change, 26 tasks, one
`sequential` constraint per requirement and no conditional constraint. Checked against the archived
copy of the plan, because the change was archived upstream while this work was in progress, so the
numbers are for the plan as it stood and not for a change that still exists.

| backend | verdict | wall clock |
| --- | --- | --- |
| `spin` | valid, 0 violations | 1.37 s |
| `spin-rs` v0.5.35, the pin | valid, 0 violations | 1.19 s |

The plan was archived upstream while this work was in progress, so it cannot be re-measured in
place; these are the numbers taken while it was live. The engine that was rejected below is faster on
the archived copy (0.43 s), which is exactly why speed alone cannot decide a pin.

The same plan in its earlier shape - 24 tasks, 18 claims, four liveness claims, and a failure block
on every task because four constraints named a failure flag - ran past 300 s under `spin-rs` while
`spin` closed it in 1.0 s. The difference between those two rows is the generator, not the plan's
size.

**Generator probe, in-process backend.** Shape: a chain of proctypes, one safety claim that holds in
every reachable state, and a failure block on the first `branched` tasks.

| tasks | branched | states, pin v0.5.35 | states on the rejected engine |
| --- | --- | --- | --- |
| 8 | 1 | 82 | 82 |
| 12 | 1 | 122 | 122 |
| 16 | 1 | 162 | 162 |
| 20 | 1 | 202 | 202 |
| 6 | 6 | 694 | 92 |
| 8 | 8 | 2 806 | 124 |

Ten states per task when one task branches; a factor of two per task when every task branches.
Before the fix the right-hand column was the cost of any plan with one conditional constraint.

**The same chain under a liveness claim, in-process backend.** Unchanged between the two pins:
the claim names the flag, so nothing about the flag may be projected away. Claim
`[] ( failed_t1 -> <> active_tN )`, default budget. The second column is the same chain with no
failure flag at all and a claim that names only a bit both fixtures share.

| tasks | with the flag | without it | elapsed (with) |
| --- | --- | --- | --- |
| 6 | 632 | 316 | 0.17 s |
| 8 | 2 552 | 1 276 | 0.94 s |
| 10 | 10 232 | 5 116 | 5.36 s |
| 12 | 40 952 | 20 476 | 29.1 s |

A factor of four per two tasks, and exactly a factor of two between the columns: this path is still
exponential on both pins, because the engine projects the state of a safety claim and not
the state of a liveness claim, so a field the claim cannot see still doubles the space. A plan whose
constraints are all IF...THEN therefore still pays it, which is why the gate measures this path
separately (T4.3) rather than trusting the safety rows above - and why that gate records the exact
pair rather than a bound.

**A claim that cannot see a flag some other claim names.** This is the only shape in which the
engine's projection of a liveness claim pays, and it is the shape veriplan produces when a plan has
several claims and only some of them name a failure flag. Two flags present, one claim naming
neither:

| tasks | pin v0.5.35 | rejected engine |
| --- | --- | --- |
| 8 | 154 | 47 |
| 12 | 234 | 67 |

The single-claim liveness rows above do not move between the pins, and that is not a defect in
either engine: the generator emits a failure flag only when some claim names the flag, so in a
single-claim model "the flag exists" and "the claim can see the flag" are the same condition and
there is nothing to project away. A fixture can only measure what its own shape exposes, and this is
the fifth time in this work that sentence has had to be written.

**The requirement this change leaves unsatisfied.**
`model-check / An unchecked constraint is reported` states that no backend runs a search without a
bound. At spin-rs `v0.5.35` that was false for the in-process liveness path: `NestedDFS` carried no
state ceiling and no depth cap, so a ten-task chain with one liveness claim ran past 45 s under
`--max-states 1` and the budget never reached the path that hung.

An engine that fixes this exists and was measured: at `v0.5.37` the same command returns in 0.006 s
and names the claim, `Property: p0 ... the liveness search stopped at the state budget of 1 before
the product space closed`. That engine is not pinned here, for the reason in the next section, so the
requirement stays false at the pin and is left as written.

The text was not weakened while it was false, and that was the point: a requirement edited to match a
defect stops being a statement about the system.

## Why the pin did not move

The newest spin-rs tag was tried and rejected, and the measurement is here so the next person does not
have to find it again. Bumping the pin turned one integration test red - `a_construct_is_never_decided_against_pan`,
on `tests/fixtures/constructs/unless_last_step_loop.pml`. Reproduced outside the harness, on the same
file, with three binaries:

| engine | result on `[] ( y <= 2 )` |
| --- | --- |
| `pan` | errors: 1 |
| spin-rs v0.5.35, the pin | errors: 1 |
| spin-rs v0.5.37 | errors: 0 |

The model is a loop whose `unless` escape is guarded by `x == 3`; `pan` refutes because the escape is
re-checked at the loop head with `x` already three, the handler runs and `y` becomes nine. The newer
engine reports the claim satisfied. A projection that loses a refutation is a false green, and a false
green is the one class of defect in this work that can make somebody ship a wrong verdict - so the
pin stays where the refutation happens, and the faster engine is not used.

Two lessons, both already learned once in this change and both worth the repetition:

- The fixture that caught this was written to catch a false green in a previous hunt. A suite of
  performance numbers would not have caught it, and neither would the engine's own parity gate.
- The pin moved for a good reason - the projection and the bound are real - and still had to move
  back. Speed is not a reason to accept a lost refutation.

## The boundary of what a state projection buys

A fixture measures only what its own shape can expose, and the two shapes here expose different
faults. Measured on three independent proctypes with nothing ordering them and one failure branch each, on
each pin and on the engine tree that came between them:

| tasks | `pan`, no reduction | pin v0.5.35 | the tree between them | rejected engine | rejected, no POR |
| --- | --- | --- | --- | --- | --- |
| 3 | 156 | 141 | 704 | 141 | 729 |
| 4 | 781 | 521 | 6 144 | 521 | 6 561 |
| 5 | 3 906 | 1 873 | 53 248 | 1 873 | 59 049 |

Read across the last three columns: with partial order reduction off, every engine agrees exactly, so
the projection changes nothing about this shape. What it cannot do is remove interleaving - N
independent processes have N! orderings of their program counters, and those counters are always
read, so no sound projection may drop them.

The fourth column is recorded because it is the reason this section exists. Running the fixture
against the engine's next tree showed the count *rising* from 141 to 704 - reduction had stopped
working - and the engine session traced it to one field answering two questions: which transitions may
commute (a property of the claim, what reduction reads) and which states are the same (a property of
the whole program, what hashing reads). Answering the second with the first collapses the state space;
answering the first with the second disables reduction. Both are unsound, in opposite directions. With
the two separated, this fixture returns to the pin's column exactly.

Two consequences for whoever reads these fixtures next:

- A chained fixture measures the state projection; a fan-out fixture measures interleaving. They are
  not interchangeable, for the same reason the two liveness directions were not.
- A plan with genuinely independent tasks stays exponential, and neither the generator change here
  nor the engine's projection addresses it. That is recorded as its own issue rather than filed later
  as a regression against this change.
- A verdict gate cannot see this class of defect. While reduction was silently switched off, every
  verdict stayed right and only the cost moved; the same 166-model parity gate was green throughout.
  The count is the measurement that catches it, which is why the gate in this change asserts counts
  and not only verdicts.

The count column that looks anomalous is neither engine being wrong: `pan` removes a process from the
state vector when it terminates, while the in-process engine carries a completion bit for every task,
so its state is strictly finer and it distinguishes completion orderings that `pan` merges. The two
counts are comparable only within one engine.

## Every gate in this change was checked by breaking the code

A test whose failure has never been observed is a guess. Each assertion below was confirmed by
reintroducing the defect it is meant to catch, watching it go red, and restoring the code.

| defect reintroduced | test | observed |
| --- | --- | --- |
| every task carries a failure flag and branch | `a_task_no_claim_names_carries_no_failure_block` | red |
| as above | `a_failure_flag_is_not_matched_as_a_prefix` | red |
| as above | `the_state_count_does_not_follow_the_unnamed_tasks` | red, naming the state cap |
| a stopped search reusing `state_budget` | `a_stopped_search_names_its_own_cause_and_its_own_cost` | red |
| `VERIPLAN_MAX_STATES` silently replaced | `a_budget_the_caller_set_is_honoured_or_refused` | red |
| the state projection removed, on the pinned engine | `the_branching_canary_matches_the_pinned_engine` | red: 92 became 694 and 124 became 2 806 when the pin moved, and the table was lowered |
| the liveness projection removed | `a_claim_blind_to_a_flag_another_claim_names_is_projected` | red: the message names the 154 and 234 this fixture explored before v0.5.37 |

The first five rows were written from expectation before being run and confirmed afterwards - which
is the defect the table is about, committed in the table. A record of a verification is not a
verification.

The last two rows came from the pin moving rather than from a hand-made mutation, and they are the
better kind: a real defect, in a real dependency, caught by a table that refused to accept the old
numbers. The table was lowered deliberately, and the test that was lowered is the one that then
guards the new value.

One assertion had to be inverted rather than lowered. The branching canary also asserted that its
fixture was exponential, which was true while the fixture was the defect. After the pin moved the
fixture became linear and the assertion failed for the right reason at the wrong time - a test that
insists its subject stay broken. The assertion now says the fixture must be linear, because the
projection is on, and names the alternative it is ruling out. That is the second time in this change
that a test which depended on a defect had to be turned around when the defect went.

The first pass at the state-count test took **ten minutes** to go red, because the broken model needs
millions of states before the default budget stops it. A canary that slow is one people switch off, so
the test was given a five-thousand-state budget.

**That fix silently destroyed the test, and only the mutation caught it.** With both sides capped, the
good model reads about two hundred states and the broken one reads exactly the cap, so the ratio
between them passes: five thousand is under four times five thousand. The mutation went green. The test
now fails on the cap being hit at all, which is unambiguous, and on an absolute bound of a thousand
states for a twenty-four-task chain. Both fire in five seconds.

Two lessons, both worth more than the code they produced:

- A bound set by judgement rather than measured at both ends is a bound that cannot fail. This change
  shipped one (a ceiling of four thousand against a measured two thousand five hundred) and had to
  replace it with exact tables.
- Making a slow test fast can make it vacuous. The cap that made this one usable is the same cap that
  made it blind, until the cap being reached became an assertion of its own.

## Open Questions

- Whether the failure block should exist at all. An IF...THEN constraint could be checked as a claim over a model that never branches, with the branch modelled in the claim instead. That changes what a verdict means and needs its own design pass; it is out of scope here.
- Whether the gate should assert states per second rather than seconds. A rate is machine-independent and would be the better assertion once `a6fr` lands and the baseline stops moving by an order of magnitude. Deferred until then.
- What the wall-clock allowance for the external backend should be. Five seconds is arbitrary and is not changed here; making it configurable without a measurement to configure it to would only move the arbitrary number.

# Proposal

## Why

A 24-task plan (the previous shape of chevron-credentials: 24 proctypes, 18 claims, four of them
liveness, 24 failure blocks) ran past 300 s under `--checker spin-rs` while `spin` closed every
claim in about one second. The same plan now finishes in 0.65 s because its constraints no longer
reference a failure flag — the slowdown is a property of the generated model's shape, not of the
plan's size, and nothing in the suite notices which shape a plan has.

Three separate defects produce that, and only one of them is in this repository:

1. The generator gives **every** task a nondeterministic failure block as soon as **any** constraint
   mentions `failed_`. Measured on a chain matching the generator exactly, one safety claim that
   does not read the flag: 694 / 2 806 / 11 254 / 45 046 states at 6 / 8 / 10 / 12 tasks, where `pan`
   stays at 37 / 49 / 61 / 73. One IF...THEN constraint in a 24-task plan costs about 16.7 M states
   per claim.
2. The budget that is supposed to bound a search does not reach the liveness path of the in-process
   engine. `--max-states 1` on a ten-task model with one liveness claim still runs past 45 s. The
   existing requirement that "no backend SHALL run a search without a bound" is therefore false
   today for every plan carrying an IF...THEN constraint, and it fails silently: the claim is
   reported as unchecked with a budget reason it never consulted.
3. The external backend's five-second per-claim fuse reports itself under the same
   `state_budget` tag as a genuine budget stop and never prints its elapsed time, so a run that
   gave up is indistinguishable from a run that finished. Comparing the two backends then compares
   one engine that completed with one that refused to continue.

## What Changes

- **Failure block per task, not per plan.** The generator asks, for each task, whether any
  constraint's LTL formula names that task's failure atom — walked structurally over
  `LtlCondition::Atom`, not matched as a substring of the rendered formula. Tasks no claim reads
  emit no block, so the flag does not enter the state vector and the branch cost disappears.
- **A stopped search reports why and for how long.** The external backend's wall-clock timeout gets
  its own outcome and its own reason, distinct from an exhausted state budget, and the elapsed time
  per claim is reported. The existing `state_budget` reason stays reserved for a budget the engine
  actually consulted.
- **The engine's per-claim budget is surfaced.** Where a backend reports that a named claim stopped,
  with states used against states allowed, the report carries those numbers for that claim instead
  of a single run-level "unchecked".
- **A stopped backend is not a conforming backend.** Comparison mode distinguishes a backend that
  stopped on its budget from one that completed, and does not count the pair as agreement.
- **The in-process engine gets a pinned gate.** A regression test asserts both state count and wall
  clock for the in-process engine on a generated plan of fixed shape, and fails when a plan carrying
  one failure block grows superlinearly in task count. The existing linear-state-space test keeps its
  engine-independent oracle; this is a second, separate assertion about the engine.
- **Consumed, not implemented here:** the engine-side fixes are the OpenSpec change
  `veriplan-perf-parity` in the spin-rs repository — claim-relative state projection, a liveness
  budget, and per-state throughput, in that order. The first two ship in a spin-rs tag this change
  does not pin: that engine also stops refuting an `unless` construct the pinned engine refutes, and
  the design records the measurement. The third fix does not exist and is not claimed.

## Capabilities

### New Capabilities

None. Every behaviour changed here already belongs to a capability.

### Modified Capabilities

- `model-check`: the failure block is emitted per task by walking the constraint's formula; a search
  that stops reports its own cause, its elapsed time and its per-claim budget, and the bound applies
  to liveness claims as well as safety claims.
- `backend-conformance`: a backend that stopped is not a backend that conformed, so the comparison
  distinguishes the two; and the in-process engine carries a pinned state-and-wall-clock gate of its
  own, separate from the model's state-space bound.

## Impact

- `src/checker/promela.rs` — failure-block predicate becomes per task and structural.
- `src/checker/spin.rs` — `PanOutcome` gains a wall-clock outcome; the reason tag and elapsed time
  are reported per claim.
- `src/checker/spin_rs.rs` — relays the engine's per-claim `states_used` / `budget` / kind instead
  of a run-level message.
- `src/checker/mod.rs` — `ConstraintSummary` / `VerificationResult` carry the per-claim numbers.
- `src/cmd_check.rs` — comparison output names a stopped backend rather than scoring it as a match.
- Tests in `src/checker/promela.rs` and `tests/` — a per-task emission test and the pinned engine
  gate.

The unchecked counts and the backend name required by `output-contract` are unchanged; the visible
difference is that one reason string stops overloading `state_budget`.

**Dependency, stated plainly:** item two and the liveness half of item three are in the in-process
engine, which this repository does not own. Until a spin-rs release carries a liveness budget, the
bound is real only for safety claims, and this change can only report that honestly. Items one, four
and five are independently shippable and are worth shipping regardless, because the per-task
predicate removes most of the exponential before the engine fix arrives and stays correct after it.

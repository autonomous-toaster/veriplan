## Context

See proposal.md for the measurements. This document records why the fix is three
emitted statements and not an engine or a model rewrite, what it costs, and what it does
not fix.

`src/checker/promela.rs` emits one proctype per task. Each body is a `do … od` with a
single guard, then three assignments — activate, complete, deactivate — then an optional
non-deterministic failure block, then `break`. The successor's guard is
`done_<predecessor> == 1`, and `find_predecessors` chains each task to the previous task
in its phase, or to the last task of the previous phase.

Because the completion flag is written before the deactivation, the guard of the
successor becomes true one statement early. The scheduler may interleave there. With N
tasks the model therefore contains the product of N two-way choices, and the reachable
state count grows geometrically: 1 276 at eight tasks, 5.2 million at twenty.

## Goals / Non-Goals

**Goals:**

- A linear plan has a linear state space, asserted by a test rather than assumed.
- Every verdict that is correct today stays correct, including the refutation of a
  requirement that contradicts the phase order.
- The model check becomes usable for the plan sizes this repository actually has.

**Non-Goals:**

- Making the engine faster. The dependency explores thirty times the states pan does and
  runs at a few thousand states per second; after this change a sixty-task plan is a few
  seconds on that engine, so speed stops being the binding constraint.
- Changing the constraint language, the verdict shape, or the set of modelled keywords.
- Replacing the model check with static analysis. That is a real option, considered in D4
  and rejected for a separate change.
- Reducing the state vector. The atoms stay; only the interleaving disappears.

## Decisions

### D1 — Publish completion last, and after the failure block

The completion flag moves after the deactivation, and after the `if :: failed … fi` block
when that block is emitted, so that "done" means "finished, and its failure status is
decided".

**Why the failure block too?** A conditional constraint reads `failed_<task>`. If the
flag were published before the failure block, a successor could start while the
predecessor's failure status was still undecided, and a claim about that status would be
evaluated in a state the plan does not permit. The unmeasured part is recorded under
Risks.

### D2 — Assert the state count, with pan as the oracle

The regression test builds a twenty-task linear plan with one ordering constraint and
asserts that `pan` explores fewer than a thousand states. A second, engine-free test asserts
the emission order itself: a task publishes completion after it deactivates, and a
successor's guard reads the predecessor's completion flag.

**Why a state count, when a state count is not evidence?** Because the claim under test is
about scale, so the count *is* the subject rather than a proxy for correctness. Every
verdict test in the suite passes both before and after the window opens; nothing else
notices. This is the same lesson as the dependency's own history, where a code path was
dropped from codegen for four releases while every behavioural test stayed green.

**Why pan and not the in-process engine?** Measured during implementation: the in-process
engine explored twenty thousand states at twelve chained tasks where pan needed sixty-one,
because a trailing `break` was emitted as its own independent transition and could be
deferred past the successor's start. Asserting the model's linearity through an engine that
is itself superlinear makes the test fail for a reason that is not the model's, which is
worse than no test at all. The two engines now agree on linear growth, but the coupling is
what we are avoiding, not the current number. The test skips when `pan` is unavailable, and
the structural test is what runs everywhere.

### D3 — Accept the exclusivity correction, and keep concurrent coverage

Moving the flag makes `AT MOST ONE` across two tasks of a *sequential* phase
structurally true. That is not a lost check: the two tasks are ordered by construction
and can never be active together, so the current refutation is an artifact of the
window. The skill has always stated that such a constraint is refutable only across a
`[concurrent]` phase; after this change the model agrees with the documentation.

The fixture that covers exclusivity is therefore split: the sequential-pair case expects
`VALID`, and a concurrent-phase case keeps expecting the refutation, so the check stays
covered rather than silently weakened.

### D4 — Not static analysis, though it is the deeper answer

Every constraint the translator emits is, at bottom, a statement about a total order over
phases: an ordering constraint either agrees with the phase chain or contradicts it, and a
conditional's consequent either lies after its trigger or does not. Both are decidable by
walking the phase structure, in milliseconds, at any plan size. Only exclusion inside a
concurrent phase needs a search.

That reframing would make the model check almost vestigial and would replace it with a
linter plus a tiny model over concurrent phases. It also changes what "verified" means,
invalidates the measurements in the published specs, and discards the engine work this
change depends on. It is the right *next* question, not this one.

## Risks

- **The position of the completion flag relative to the failure block is unmeasured.**
  Every model measured for this change had no conditional constraint, so no failure block
  was emitted. The recommendation follows from what "done" should mean, not from a
  measurement, and this construct has produced three separate wrong verdicts in the
  dependency already. Task 1.2 therefore carries its own fixture, and the plan for it is a
  conditional constraint over a failure flag.
- **A state-count test is a threshold.** A plan that legitimately needs more than a
  thousand states at twenty tasks would trip it. The mitigation is the shape: a linear
  chain with one ordering constraint, not a corpus plan.
- **Fixtures will move.** An expectation that reads `INVALID` for a sequential-pair
  exclusion becomes `VALID`. Anyone reviewing the diff without this document will read
  that as a regression, so the fixture change names this decision.
- **The published scale numbers in the skill go stale** the moment the change lands. Task
  2.2 rewrites them from a fresh measurement rather than adjusting them by estimate.

## Migration Plan

1. Reorder the emitted statements and place the flag (phase 1). The suite must stay green
   apart from the one fixture whose expectation is meant to move.
2. Move that fixture and add the concurrent variant in the same commit, so the check is
   never uncovered.
3. Add the state-count test last, so it records the achieved number rather than a target.
4. Refresh the skill's scale section from measurement, and close the tracked issue.

## Self-check

- Every requirement in this change is a forward ordering over tasks that exist in its own
  `tasks.md`, so it survives the parser this change is about.
- The change does not rely on any behaviour it removes: no requirement here claims that a
  sequential-pair exclusion refutes.
- The change's own plan is small enough to model-check under the current shape, so its
  gate is meaningful before the fix as well as after.

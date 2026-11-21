## Why

The Promela model veriplan emits has a state space the plan does not have. Measured
with pan as the oracle on a plain linear chain, one ordering constraint, nothing
violated:

| tasks | states (pan) |
|---|---|
| 8 | 1 276 |
| 20 | 5 242 876 |
| 62 | not measurable |

A twenty-task plan that has exactly one legal execution order takes five million
states to check, and a plan of twenty-six tasks — this repository's own archived
change — cannot be checked at all: it reports `UNKNOWN` after exhausting the
default budget. That is why the model check has been unusable above roughly ten
tasks, and why a previous change's gate could only be satisfied by hand-waving.

The cause is one assignment in the wrong place. Every task body is emitted as

```
active_tN = 1;
done_tN   = 1;      /* the successor's guard reads this */
active_tN = 0;
```

The successor's guard is `done_prev == 1`, so it may start as soon as `done` is
set — while the predecessor is still deactivating and before its own failure block
runs. Every task boundary therefore opens a small interleaving window, and the model
takes the *product* of those windows. Moving the completion flag to the end of the
body closes the window and makes the model linear.

Measured on hand-modified models, with the reordering and nothing else changed:

| tasks | current | reordered |
|---|---|---|
| 8 | 1 276 | 41 |
| 20 | 5 242 876 | 101 |
| 62 | not measurable | 311 |

## What Changes

- **A task publishes completion after it has finished.** The completion flag is emitted
  after the deactivation, and after the failure block when one is emitted, so a successor
  cannot start until the predecessor is complete.
- **The linear state space is asserted.** A regression test checks that a twenty-task
  plan explores fewer than a thousand states. The number is the subject of the test, not
  evidence of correctness: nothing else in the suite would notice the window reopening.
- **One documented behaviour changes.** `AT MOST ONE` between two tasks of a sequential
  phase stops refuting. The two tasks are ordered by construction and can never overlap;
  today's refutation is an artifact of the window. The skill already states that such a
  constraint is refutable only across a `[concurrent]` phase, so the model becomes true
  to the documentation instead of the documentation being adjusted to the model.
- **The recorded scale numbers are replaced.** The skill carries a scale section with
  8-task and 10-task timings taken from the old shape; those numbers are re-measured.

## Capabilities

### New Capabilities

- `model-scale`: the state space of a generated model SHALL grow linearly in the number
  of tasks when the plan is a linear chain, and the suite SHALL assert that bound rather
  than a verdict.

### Modified Capabilities

- `model-check`: the Promela generator emits a task's completion flag after the task
  is complete, and the model's state space is linear in a linear plan.
- `backend-conformance`: recording a scale measurement becomes asserting one.

## Impact

- `src/checker/promela.rs` — the order of three emitted statements, and the position of
  the failure block relative to the completion flag.
- `src/checker/mod.rs` — a regression test bounding the state count of a twenty-task plan.
- `tests` and fixtures — the exclusive fixture over a sequential pair no longer refutes;
  its expectation moves from `INVALID` to `VALID`, and a concurrent-phase variant keeps
  the refutation covered.
- `skills/veriplan-spec/SKILL.md` — the scale section's numbers and the sentence that says
  `AT MOST ONE` is refutable only across a concurrent phase.
- Behaviour: plans that were `UNKNOWN` at the default budget become checkable. No verdict
  changes except the sequential-pair exclusivity described above.

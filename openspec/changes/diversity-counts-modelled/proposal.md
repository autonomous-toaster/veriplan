# Proposal

## Why

A peer session shipped a plan with 30 ordering constraints and read this line as an unfixable finding:

```
[INFO] low_diversity
  Constraint distribution: sequential(30). Consider adding other constraint types for stronger verification
      Fix: Add exclusive (mutex), conditional (if-then), or concurrent constraints
```

Three defects stand behind that report, and the first two are measurable.

1. **The tally counts prose, not the model.** `check_diversity` classifies each requirement's text and
   adds the category to the distribution. A requirement that yields no formula therefore counts toward
   diversity while adding no verification. Measured: a plan of three sequential requirements reports
   `low_diversity`; appending one unmodelled `IF a client supplies a <slug> THEN ...` requirement makes
   the INFO disappear and leaves the checked constraint count unchanged. A metric that is satisfied
   without verification is wrong, not merely noisy.
2. **The fix hint does not say what a non-sequential constraint needs.** The atoms of the model are
   tasks (`active_T11`, `done_T11`, `failed_T11`). `IF ... THEN`, `AT MOST ONE` and `CONCURRENTLY`
   are extracted into a formula only when the body cites two task ids. The hint names the keywords and
   omits the precondition, so an author who follows the hint literally, writing behaviour rather than
   task order, reaches a dead end and sees the same INFO again.
3. **The report does not name the missing argument.** Such a body is degraded to the generic
   `pattern_ungrounded` message ("add task IDs for model verification"), which does not say that a
   conditional needs a trigger and a consequent, or that `AT MOST ONE` needs the pair kept apart.

## What Changes

- **The distribution counts the constraints the model carries.** Only a constraint that produces an LTL
  formula enters the tally, so the advisory describes what was checked and can be satisfied only by
  adding verification.
- **The hint states the precondition and the standing of the line.** A non-sequential constraint SHALL
  cite two task ids of this change; the message names a shape that does. The line stays advisory: a
  constraint the plan does not need is worse than the INFO.
- **The missing argument is named per category.** `Exclusive`, `Conditional` and `ConcurrentEvents`
  each report which two tasks the body failed to cite, keyed to the keyword the author wrote.

Consumed, not implemented here: extracting non-sequential constraints from behaviour needs atoms the
plan does not declare (a `slug_supplied` state). Until such atoms exist, the honest answer to
"extract this or drop the INFO" is neither: report what the model carries, and say what would change it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `convertibility-check`: the diversity advisory is computed from modelled constraints; the
  `pattern_ungrounded` finding for a non-sequential keyword names the argument that is missing.

## Impact

- `src/checker/checks.rs` — `check_diversity` tallies `translator::translate_all` and skips constraints
  without a formula; the fix hint is rewritten.
- `src/checker/checks/classifiability.rs` — a requirement whose non-sequential keyword lacks its two
  task ids reports the missing argument instead of the generic message.
- Tests in `src/checker/checks/checks_tests.rs`.

# Design

## The measurement

Two fixtures, both run with the pre-change binary and again after it.

| fixture | before | after |
|---|---|---|
| 3 sequential requirements | `low_diversity`, distribution `sequential(3)` | same |
| the same plan plus one `IF ... THEN` body citing no task id | no `low_diversity` | `low_diversity`, `sequential(3)` |
| 3 sequential + 1 conditional + 1 exclusive, each citing two task ids | 5 modelled constraints, no INFO | same |
| 30 sequential constraints over 31 tasks | `VALID`, `sequential(30)` | `VALID`, `sequential(30)` |

The second row is the defect: the advisory disappeared while the model gained no constraint. The
third row is why the report's diagnosis was wrong — `IF ... THEN` and `AT MOST ONE` are extracted,
and become a formula as soon as the body cites two task ids.

## D1: the tally reads the model, not the prose

`check_diversity` classified each requirement's text. It now iterates `translator::translate_all` and
skips every constraint without a formula. One source of truth decides what is checked and what the
distribution says, so the two cannot drift.

Consequence: `non_formalizable` and `pattern_ungrounded` no longer appear in the distribution, because
neither ever carries a formula. The threshold stays at three constraints.

## D2: the hint states the precondition

The atoms of the model are tasks, so every non-sequential category needs two task ids. The hint now
names a shape that cites two and says the line is advisory. The alternative — leaving the hint as a
list of keywords — is what produced the report: an author writes behaviour, gets no constraint, and
reads the same INFO again.

## D3: the missing argument is named per category

A body whose keyword is `Exclusive`, `Conditional` or `ConcurrentEvents` and which cites fewer than two
task ids is now reported as `PatternUngrounded` with a message naming what is missing: a trigger and a
consequent, the pair kept apart, or two tasks of one concurrent phase. This also reports the one-task-id
case, which previously classified as modelled and then silently carried no formula. A body with no pair
keyword keeps the generic message.

## Rejected

- **Extract non-sequential constraints from behaviour.** `IF a client supplies a <slug> THEN ...` has
  no atom in a task-order model. Inventing a `slug_supplied` proposition would state a fact the plan
  does not declare, and a formula built from invented atoms cannot refute anything the plan says.
- **Suppress the INFO when every task pair is already ordered.** A total order makes an added
  mutex redundant or contradictory, but it does not make a conditional redundant (a failure branch is
  still meaningful), and a global invariant over the ordering would silently change as tasks are added.
  Suppressing an advisory on a global property hides a legitimate suggestion.
- **Drop the INFO.** It is the only signal that a plan verifies one kind of relation. The line costs
  nothing and never blocks.

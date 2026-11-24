# Construct fixtures, and the cell each one occupies

Every model here is checked against `pan` by `tests/construct_verdicts.rs`, which records pan's answer
beside the file. The engine may refuse a construct or decline a claim - both are "no verdict" - but it
may not decide a model and disagree with pan.

## The rule these exist for

**A guard evaluated at a re-entry point is a different reader from a guard evaluated once in body
order.**

A body statement reads a variable at one point in one execution order. A guard at a re-entry point -
a loop head, an `unless` escape, a label target, an assertion, a boundary - is evaluated again and
again, in places the emitter's dependency metadata does not reach unless it walks them. An optimiser
that keys state identity on "what reads this field" therefore drops the field, and the guard can never
become true. The consequence changes from "slow" to "wrong" the moment identity depends on the same
metadata reduction does.

That has produced **three false greens across two repositories**, every one of them found by a fixture
that existed before the optimisation did:

| where the read sat | what was reported clean |
| --- | --- |
| an `unless` escape whose handler carries statements | `unless_last_step_loop.pml` |
| an assertion over a field no claim names | `goto_loop_control.pml` |
| an assertion inside a `Stmt::Block` | found by sweeping this matrix on the engine side |

## The matrix

Rows are constructs, columns are where the read is evaluated. A fixture per cell, because a fix closes
the cell that failed and leaves its neighbours open - three releases in a row each closed one cell.

| construct | position: body first step | body repeat | block | label | handler / escape | assert |
| --- | --- | --- | --- | --- | --- | --- |
| `unless` | `unless_second_step*` | `unless_midbody*` | `unless_last_step_block` | - | `unless_multi_handler`, `unless_refutes`, `unless` | - |
| `goto` | `goto_loop_partial` | `goto_loop_control` | - | `goto_loop_partial`, `goto_loop_control` | - | `goto_loop_control` |
| `assert` | `assert_bool_equals_zero`, `assert_not_bool_false` | **open** | `wrap_assert` | **open** | **open** | - |
| claim atoms | `bare_implication`, `boxed_implication`, `response_bare_vacuous` | `live_cmp_atom` | - | - | - | - |
| `never` automaton | `never_accept_*` | `never_cycle_*` | - | - | - | `never.pml` |

`-` means the cell does not exist; **open** means it does not exist and the rule above says it could.

## Adding one

1. The model must be decided by `pan`, and pan's answer recorded in `MODELS` in
   `tests/construct_verdicts.rs`. Read the note there before choosing an expectation: for acceptance
   cycles, plain `pan` reports 0 for both members of a pair and only `pan -a` separates them.
2. The fixture must fail when the corresponding fix is reverted. An expectation nobody has watched
   go red is a guess.
3. Add one cell, not one bug. The neighbouring cells are where the next release will be wrong.

## Why the gate is wider than the change

The change that motivated most of these was about state counts; the gate that stopped it twice
predates it and knows nothing about it. A gate scoped to a diff cannot know which constructs the diff
now depends on. **The correctness gate has to be wider than the change.**

The counts are only worth having once the verdicts are right.

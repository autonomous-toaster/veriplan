---
name: veriplan-spec
description: Write OpenSpec changes that pass `veriplan check`. Rules for tasks.md and spec.md, how to read the verdict, and the two limits that will otherwise waste a session.
---

# Veriplan-compliant OpenSpec

veriplan turns each requirement into a temporal constraint over the tasks in
`tasks.md` and model-checks the plan. Requirements describe **task ordering,
never code behaviour**.

## What a verdict looks like

```
Plan: <change> — ✓ VALID [spin]
  Satisfied: 3 | Violated: 0 | Unmodelled: 0 | Unchecked: 0 | Total: 3
```

- `✓ VALID` — every constraint was modelled and held. This is the only pass.
- `⚠ UNKNOWN` — nothing was proved or refuted: all constraints unmodelled, or
  every search hit the budget. **Not a pass.** The `skip_reason` says which.
- `✗ INVALID` — a constraint was refuted, or convertibility blocked.
- `[spin]` / `[spin-rs]` — the engine that produced the verdict. Two engines
  give the same answer on a correct plan; if they disagree, suspect the plan.

Read the four counts, not just the tick. `Unmodelled > 0` means part of the
spec was never checked, and `Total = Satisfied + Violated + Unmodelled + Unchecked`.

## Non-negotiable core

A requirement is one temporal relation between two tasks of this change:

```
T2.1 SHALL complete BEFORE T3.1 SHALL start.
```

- Reference ids **verbatim and as whole identifiers**: `T1.10` is not `T1.1`.
  Plans commonly have tasks 5.10 and 7.12; writing `T5.1` there references a
  different task.
- A cited id that no task of this change defines is `grounding_absent_task`,
  which names it. Citing a task from an archived change does not work — the
  signature is built from this change's `tasks.md`.
- One relation per requirement. Two sentences that each begin with a task id is
  `pattern_multi_clause` even when the second has no keyword; two temporal
  keywords is `grounding_multi_keyword`. Split either way.

## The four modelled keywords

These produce a formula and can be checked:

| keyword | shape | note |
|---|---|---|
| `BEFORE` / `AFTER` | `T1.1 SHALL ... BEFORE T1.2 SHALL ...` | the workhorse |
| `AT MOST ONE` | `AT MOST ONE of T1.1 and T1.2 SHALL ...` | refutable only across a `[concurrent]` phase |
| `IF ... THEN` | `IF T1.1 fails THEN T1.2 SHALL ...` | the consequent must come later in the plan, or it is refuted |
| `CONCURRENTLY` | inside a `[concurrent]` phase | otherwise unmodelled |

## The two that are not modelled

`ALWAYS` and time-window phrasing (`within`, `window`, `between ... and`) carry
no encoding in a task-order model. They are reported as **Unmodelled**, never
counted as satisfied, and a plan whose only constraints are unmodelled reports
`⚠ UNKNOWN`. Do not use them to satisfy coverage or to add variety.

```
BAD   T1.1 SHALL ALWAYS hold the header.        -> Unmodelled, checks nothing
GOOD  T2.1 SHALL complete BEFORE T3.1 SHALL start.
```

## tasks.md rules

- Unique `N.M` ids under `## N. Group` headings. Execution order is heading
  order, then task order inside the heading.
- `[concurrent]` on a heading makes its tasks run unordered. Any `## N. ...`
  heading works — the word "Phase" is not required.
- One instruction per description, no ambiguous pronouns. Both are blockers in
  Strict, which is the default. `Install the engine and wire it` is two
  instructions and an unresolved "it".
- Task descriptions are grounding aliases: a requirement may cite a task by its
  id (preferred) or by repeating its whole description. One shared word is not
  an alias.

## Scenario rules

- `#### Scenario:` — exactly four `#`.
- `- **WHEN** <condition>`, then `- **THEN** <outcome with SHALL/MUST>`,
  optionally `- **AND**`.
- No ambiguous pronouns in steps.

## Two findings that are not your fault

- `low_diversity` (info): every constraint is `BEFORE`/`AFTER`. **Do not add an
  `AT MOST ONE` or an `IF...THEN` to silence it.** A constraint the plan does
  not need is a constraint nobody can maintain, and the info line costs nothing.
- `task_not_covered` asks for a task id in a requirement body. If the task is
  genuinely not constrained, cite it in a real relation to a neighbour rather
  than inventing one.

## Coverage

- At least one `SHALL`/`MUST` requirement, else `no_requirements`.
- Every task cited by at least one requirement body, else `task_not_covered`.
  A task id that appears only in a `WHEN`/`THEN` step does not count.
- `skip_specs: true` does not exempt a change.

## Scale: read this before iterating

The model is one process per task, so its state space grows fast. Measured with
`--checker spin-rs`: 8 tasks ≈ 10 s, 10 tasks ≈ 130 s, and a 26-task plan reports
`⚠ UNKNOWN` after exhausting the default budget.

- Iterate with `veriplan check <change> --phase convertibility`; it is instant
  and catches everything except the model check.
- Above roughly 10 tasks, `UNKNOWN` is the tool's limit rather than a fault in
  your spec. Do not keep rewriting requirements chasing `✓ VALID`; say so
  instead.
- `VERIPLAN_MAX_STATES` raises the in-process budget at the cost of time.

## Checklist

- [ ] ids referenced verbatim, whole, and belonging to this change
- [ ] one relation per requirement; no second task-initial sentence
- [ ] every keyword is one of the four modelled ones
- [ ] every task cited by a requirement body
- [ ] scenarios: four-`#` header, WHEN, THEN with SHALL/MUST, no pronouns
- [ ] `--phase convertibility` clean, then the full check, then read the counts

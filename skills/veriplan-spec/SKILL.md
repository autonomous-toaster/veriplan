---
name: veriplan-spec
description: Write OpenSpec changes that pass `veriplan check` on the first run. Concise, complete rules for tasks.md and spec.md so a change validates. Use when creating or editing the specs of an OpenSpec change.
---

# Veriplan-compliant OpenSpec

A change validates when veriplan can turn every requirement into a temporal
constraint on **real tasks** and model-check it. Requirements describe **task
ordering, never code behavior**.

## Non-negotiable core

Every requirement is exactly one temporal ordering between two tasks that exist
in tasks.md:

```
T2.1 SHALL complete BEFORE T3.1.
```

## Write order

1. Write `tasks.md` first — the tasks and their IDs.
2. Write `spec.md` requirements and scenarios referencing those IDs.
3. Run `veriplan check <change>` (defaults to Strict); fix every finding; repeat
   until it reads `✓ VALID`.

## tasks.md rules

- Unique `N.M` IDs (`1.1`, `2.3`) under `## N. Group` headings.
- `[concurrent]` belongs on a **`## Phase N: Name [concurrent]`** heading (must
  start with "Phase").
- Description = **one instruction per sentence, no ambiguous pronouns.** Both
  are **blockers in Strict (the default)**.
  - BAD: `Install the engine and wire it` (two instructions, "it" unresolved)
  - GOOD: `Install the engine` / `Wire the drive` (split into separate tasks)

## spec.md requirement rules

- Pattern: `T<id> SHALL <predicate> <KEYWORD> T<id> SHALL <predicate>.`
- Exactly **ONE** temporal keyword per requirement:
  `BEFORE`, `AFTER`, `CONCURRENTLY`, `IF…THEN`, `ALWAYS`, `AT MOST ONE`.
- One keyword, one relation, per requirement. A requirement with two temporal
  clauses is a `grounding_multi_keyword` blocker — **split into one requirement
  per clause.**
  - BAD (one requirement, two relations): `T1.1 SHALL run BEFORE T2.1. T3.1 SHALL run AFTER T2.1.`
  - GOOD (split): requirement A `T1.1 SHALL run BEFORE T2.1.` + requirement B `T3.1 SHALL run AFTER T2.1.`
- Reference real IDs verbatim (`T1.2` for task `1.2`); referenced tasks must
  exist.
- No comma-separated task lists. For concurrency, put tasks in a `[concurrent]`
  phase and state a two-task relation.
- MUST NOT describe behavior/properties (`the function SHALL find all atoms`).
  Rewrite as a task ordering.

## Scenario rules (spec.md)

- `#### Scenario:` — exactly **4** `#`.
- Steps: `- **WHEN** <condition>` then `- **THEN** <outcome carrying SHALL/MUST>`.
  Optional `- **AND**`.
- THEN must contain SHALL/MUST.
- Avoid ambiguous pronouns in steps (Strict blocker).

## Coverage

- At least one `SHALL`/`MUST` requirement (else `no_requirements` blocker).
- Every task referenced by ≥1 requirement (else `task_not_covered` warning —
  still fix it to pass cleanly).
- **`skip_specs: true` does not exempt a change.** veriplan still requires
  convertible requirements that cover the tasks — write a spec even for
  tooling/refactor changes.

## Checklist

- [ ] tasks: unique `N.M` IDs; one-instruction descriptions; no pronouns; `[concurrent]` on `## Phase`
- [ ] each requirement: one task-pair, one temporal keyword, existing IDs
- [ ] no requirement mixes two temporal clauses
- [ ] every task referenced by ≥1 requirement
- [ ] every scenario: `#### Scenario:` header, `- **WHEN**`, `- **THEN**` with SHALL/MUST
- [ ] `veriplan check <change>` reports `✓ VALID`

## Workflow reminder

Fix findings in order (they name the exact issue and location), then re-run
`veriplan check`. A clean change also keeps its own spec advisory-clean
(no passive voice fights OpenSpec grammar; leave `SHALL be …` untouched).

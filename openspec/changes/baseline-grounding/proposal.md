## Why

The grounding signature is built from the active change's `tasks.md` alone, so a requirement that
modifies a published requirement cannot cite the task that originally grounded it. Reproduced with the
published body, verbatim, inside a delta:

```
grounding_absent_task: cites T2.1 — not a task of this change
  suggestion: Cite a task this change defines, or add the task to tasks.md
```

That is what made a requirement get dropped from a change in this repository, and what a downstream
repository recorded as debt it could not pay. A delta cannot re-ground a published body, so it cannot
touch one — which means a reworded requirement is out of reach of the tool that exists to check it.

The obvious fix, treating the archive as a vocabulary of task ids, does not survive measurement:

| | |
|---|---|
| distinct ids across 24 archived changes | 94 |
| ids defined by more than one change | 54 |
| cited ids in published bodies that are ambiguous | 37 of 46 |
| `T1.1` defined by | 22 changes |

Binding an id would also break the model layer: `generate_promela` declares atoms only for the plan's
own tasks, so a claim citing an archived id yields `undeclared variable`, `spin -a` fails, and the run
reports an opaque "Model check error".

Measured the other way, keyed by the *requirement* rather than the id, resolution is total:

| | |
|---|---|
| published requirements with a citation | 111 |
| resolved to exactly one archived change | 100 |
| several candidates, resolved by archive order | 11 |
| unresolved | 0 |

## What Changes

- **Every requirement carries its provenance.** The grounding change of a published requirement is the
  latest archived change whose delta contains that requirement and whose task list defines every id the
  body cites. Archive order is the date prefix on the directory.
- **A body grounds against the active change's tasks and its provenance change's tasks.** The ids
  resolve, so a typo is caught and a historical citation is no longer reported as a missing one.
- **A constraint with no live argument is classified historical.** It carries no formula, it is
  reported, and it is counted separately — never blocking, because referring to completed work is not a
  fault in the plan.
- **A constraint mixing a live task with history is reported**, not failed. The live half cannot be
  modelled without the historical atom in the model, so the constraint is historical and the foreign
  ids are named.
- **A requirement with no recorded provenance is reported as such**, which is a different fact from a
  cited id being absent.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `grounding-check`: the signature is built from the active tasks and the requirement's provenance
  change; a mixed or historical citation is reported; a requirement with no provenance is reported.
- `model-check`: a constraint is modelled, unmodelled, unchecked or historical, and the total
  accounts for the fourth.

## Impact

- `src/input/mod.rs` — locate the archive and read archived deltas and task lists.
- `src/cmd_check.rs` — resolve provenance once and pass it down as a value.
- `src/checker/convertibility.rs` — ground against the active tasks plus the provenance change's.
- `src/ir/mod.rs` — `Requirement.grounded_in`, following the precedent of `category` being
  rewritten during convertibility.
- `src/translator/mod.rs` — mark a constraint historical and give it a reason.
- `src/checker/mod.rs`, `src/annotator/mod.rs` — the fourth count in the verdict and the JSON.
- Behaviour: a plan that is `INVALID` today because a published body cannot be re-grounded becomes
  checkable, with the constraint reported as historical. This is the point of the change.
- Not changed: the model, the constraint language, and every verdict that does not involve a
  historical citation.

## Non-goals

- Merging the archive into the signature as an id namespace. The measurement above is why.
- Modelling archived tasks as completed constants: an ordering against history is tautologically true,
  and a tautology counted as a proof is the defect this tool was just rid of.
- Delta-header parsing (`ADDED`, `MODIFIED`, `REMOVED`). A *new* requirement citing history is
  therefore classified historical rather than rejected. That is honest — the constraint is about
  history — and delta parsing is the change that would let the two cases be told apart.
- Caching or indexing provenance. A full scan of 58 delta files and 24 task lists measures 6.5 ms.
- Merged-model checking of published specs plus a delta.

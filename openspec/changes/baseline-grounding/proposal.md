## Why

The grounding signature is built from the active change's `tasks.md` alone. A requirement that
modifies a published requirement therefore cannot cite the task that originally grounded it,
because that task belongs to an archived change. The finding says:

```
Requirement 'T2.6 SHALL publish the port BEFORE T1.1 SHALL rename the noun.'
  cites T2.6 — not a task of this change
```

The author did cite a real task, and the tool cannot see it. This is the blocker that stopped a
downstream repository from correcting two published specs: the honest route would have been a delta
on an existing capability, but a delta's body has to carry the original change's task ids, and those
are ungroundable by construction. The debt was recorded and left, which is the outcome this tool is
supposed to prevent.

Two things are missing, and they are separable:

1. **The archive is not a source of constants.** `signature_from_planir` iterates `plan.tasks` and
   nothing else, although the archived task lists are on disk — `openspec/changes/archive/*/tasks.md`,
   retained by the archive in every repository checked.
2. **A cited id is either known or unknown, never owned.** There is no path from an id to the change
   that defines it, so the finding cannot say which change would have to be cited instead.

## What Changes

- **The signature includes archived task constants.** Every archived change's tasks contribute a
  constant, annotated with the change that defines it, so a body citing an archived task grounds.
- **Ownership is reported.** A cited id that only an archived change defines is reported with that
  change named, instead of an instruction to cite a task of this change.
- **A constraint grounded only on archived tasks is reported.** Grounding on the archive means the
  ordering refers to work that is already complete; the constraint is still checked, and the report
  says so, so a reader can tell a live ordering from a historical one.
- **The project root reaches the grounding check.** It is resolved once in `cmd_check` and threaded
  through the convertibility check into the grounding check.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `grounding-check`: the signature is built from the active change's tasks and the archived task
  lists; a cited id is resolved to an owner; a constraint grounded only on archived tasks is
  reported.

## Impact

- `src/input/mod.rs` — resolve the archive directory beside the change directory.
- `src/checker/mod.rs`, `src/checker/convertibility.rs` — thread the project root into the
  convertibility check and from there into grounding.
- `src/grounding/mod.rs` — scan archived task lists, annotate constants with an owner, and extend
  the diagnosis.
- `src/cmd_check.rs` — supply the root it already resolves.
- Behaviour: a plan that was blocked with `grounding_absent_task` for citing an archived id now
  grounds, and its finding disappears or becomes an ownership note. Plans that cite an id no change
  defines are unchanged.

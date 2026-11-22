## Why

The grounding signature is built from the active change's `tasks.md` alone, so a requirement that
modifies a published requirement cannot cite the task that originally grounded it. The finding says
`cites T2.6 — not a task of this change`, which is true and useless: the author cited a real task and
the tool cannot see it. A downstream repository abandoned two spec corrections because of it.

The obvious fix — make the archive a source of constants — does not survive measurement. Across this
repository's 24 archived changes:

| | |
|---|---|
| distinct task ids | 94 |
| ids defined by more than one change | **54** |
| T1.1, T1.2, T1.3 defined by | **22 changes each** |
| capabilities touched by more than one change | 9 |

Task ids are `N.M` and restart in every change; that is the design, not an accident. So "the archive
is a vocabulary" is false: a flat namespace would make `T1.1` mean twenty-two different things, and
the alias sets of twenty-two task descriptions would attach to one constant — reintroducing, at
archive scale, the incidental-alias contamination that was removed from `build_aliases` earlier.

Binding would also be unsafe at the model layer. `generate_promela` declares atoms only for the
plan's own tasks, so a claim citing an archived id produces `undeclared variable: done_t2_6`,
`spin -a` fails, and the plan reports an opaque "Model check error" instead of either a verdict or
the current, accurate diagnosis.

What the archive *can* supply is provenance: which change defines an id. That is enough to turn the
misdiagnosis into actionable guidance, and it needs no model change and no new tautology.

## What Changes

- **A cited id is resolved to its owners in history.** For a requirement, the archived changes that
  touched its capability are scanned, and a cited id is matched against their task lists.
- **The finding names them.** `T2.6 is defined by change X, which is closed` replaces `not a task of
  this change`. A rename of a published requirement is work, so the guidance points at making it its
  own change with its own tasks — which is what the downstream repository concluded unaided.
- **A non-distinguishing id is named as such.** An id that twenty-two archived changes define is
  reported with that count, because the useful message is not "cite a real task" but "this id does
  not identify a task".
- **The project root reaches the grounding check**, resolved once in `cmd_check` and threaded through
  the convertibility check.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `grounding-check`: a cited id is resolved to the archived changes that define it, and the finding
  reports them instead of asking for a task id reference.

## Impact

- `src/input/mod.rs` — locate the archive directory beside the change directory.
- `src/checker/mod.rs`, `src/checker/convertibility.rs` — thread the project root into the
  convertibility check and from there into grounding.
- `src/grounding/mod.rs` — scan the archived task lists of a capability, build an id-to-owners map,
  and extend the diagnosis.
- `src/cmd_check.rs` — supply the root it already resolves.
- Behaviour: a plan blocked on a historical citation gets a diagnosis that names the owning change.
  Grounding never succeeds on a task this change does not define, and the model is unchanged.
- Not changed: the signature, the model, and the set of constraints that can be checked.

## Non-goals

- Merging the archive into the signature. The measurement above is the reason.
- Modelling archived tasks as completed constants. It would make a historical ordering checkable and
  tautological at once, and a tautology counted as satisfied is the defect this tool was just rid of.
- Delta-header parsing (`ADDED`, `MODIFIED`, `REMOVED`). It is the cleaner rule for deciding when a
  historical citation is legitimate, and it is a change of its own.
- Merged-model checking of published specs plus a delta.

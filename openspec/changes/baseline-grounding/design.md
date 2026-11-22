## Context

See proposal.md. Grounding builds a `Signature` (groundcontrol) from the plan's tasks and grounds
each requirement statement against it. A constant is a task id; the grounder matches ids and
aliases, and reports `absent_identifiers` for constant-shaped tokens the signature does not declare.

The archive is not consulted anywhere in that path, although `openspec/changes/archive/*/tasks.md`
exists in every repository checked (22 of 23 changes in this repository, 1 of 1 in the downstream
one that hit this). The tasks are complete, but their ids are the vocabulary the published specs were
written in.

## Goals / Non-Goals

**Goals:**

- A requirement may cite a task id that an archived change defines, and ground.
- The ownership of a cited id is available to the finding, so the message can name the change.
- A constraint that only the archive grounds is visible as such in the report.

**Non-Goals:**

- Model checking the archive. Archived changes are not checked as changes; this change only reads
  their task lists as a vocabulary.
- Parsing delta headers (`ADDED`, `MODIFIED`, `REMOVED`). That would let the tool distinguish a
  requirement being modified from one being added, which is the cleaner rule for deciding whether an
  archived citation is legitimate. It is a separate change, and this one does not need it to remove
  the blocker.
- Merged-model checking (published specs plus delta in one state space). Also separate, and blocked
  on the corpus being free of unmodelled constraints first.

## Decisions

### D1 — Archived constants are bindable, not merely known

The alternative is to keep the signature as it is and use the archive only to improve the message:
"T2.6 is defined by change X". That resolves the misdiagnosis but not the blocker — the author still
cannot write the sentence that describes their change, and the downstream repository still has to
abandon the correction or invent a task.

So archived tasks become real constants, and the slot binding works exactly as it does for a live
task. The guard is D3.

**Cost:** a new requirement could ground on an archived task and pass while referring to work nobody
will do in this change. That is a real risk and it is why D3 exists rather than being an optional
nicety.

### D2 — Every constant carries its owner, and the active change's constants say so

`ConstantDef` is groundcontrol's type and is not ours to extend, so ownership lives beside the
signature: a map from constant name to owning change. Constants from the active change map to the
active change, so the report can distinguish "grounded here" from "grounded in history" without
consulting the archive again.

### D3 — A constraint grounded only on archived tasks is reported

If none of a constraint's arguments come from the active change, the ordering it states is between
pieces of work that are already complete. The constraint is still modelled and checked — the model
represents the plan's order, and the archived task's completion flag is not in it, so the formula
will be evaluated against tasks that this change does define. The report names the situation so a
reader can tell a live ordering from a historical one.

**Why report rather than block:** blocking would restore the original blocker with a different
message. Reporting gives the author the fact and leaves the decision.

### D4 — The root is threaded, not rediscovered

The grounding check has no filesystem context today: it receives a `PlanIR` and a strictness. The
project root is already resolved in `cmd_check` for input resolution, so it is passed down rather
than re-derived from paths inside the plan, which are relative to the change directory and differ
between OpenSpec, directory and single-file modes.

The threading touches five signatures. That is the cost of not having a global, and it is the reason
this is a change rather than a patch.

## Risks

- **An archived constant shadows a live one.** Task ids are `N.M` and repeat across changes — that is
  the point. If the active change and an archived change both define `T2.6`, the active one must win,
  or a live reference would bind to history. The active change's constants SHALL take precedence, and
  the merge order is the guarantee.
- **The archive grows without bound.** A repository with hundreds of archived changes contributes
  hundreds of constants, all of them candidates for alias matching. The mitigation is that only ids
  and whole descriptions become aliases, so the match surface grows linearly in archived tasks, not
  combinatorially.
- **A stale archive.** A task id that an archived change defines but that was later renamed stays in
  the vocabulary. That is correct — the published spec still cites it — and the ownership note names
  the change whose task list is being relied on.

## Migration Plan

1. Resolve and pass the root (task 1.x). No behaviour change yet; the signature is unchanged.
2. Add the archived constants and the ownership map (task 1.2, 1.3). The corpus must not move except
   where a citation was previously ungroundable.
3. Extend the diagnosis and the report (task 2.x).
4. Re-run the corpus and record the change (task 3.3).

## Self-check

- Every requirement here is a forward ordering over tasks in this change's own `tasks.md`.
- The change does not require reading the archive to write its own specs, so its gate is meaningful
  before and after.
- D1's risk is answered by D3, and the precedence rule in Risks is the one thing an implementer
  could get subtly wrong, so it is named as such rather than left to the merge.

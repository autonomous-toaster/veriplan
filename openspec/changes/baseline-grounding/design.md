## Context

See proposal.md for the measurement that rules out the obvious fix. This document records what the
archive can and cannot be, and why the smaller answer is the correct one.

Grounding builds a `Signature` from `plan.tasks` and grounds each requirement statement against it.
The archive is not consulted, although `openspec/changes/archive/*/tasks.md` is retained everywhere
checked (24 of 24 changes here, 1 of 1 downstream).

## Goals / Non-Goals

**Goals:**

- A citation of an archived task id produces a diagnosis that names the owning change.
- An id that many changes define is reported as non-distinguishing rather than as a missing id.
- The project root reaches the grounding check without being re-derived from relative paths.

**Non-Goals:**

- Grounding on archived tasks. See D1.
- Changing the model, the signature, or which constraints are checkable.
- Delta-header parsing, merged-model checking.

## Decisions

### D1 — The archive is provenance, not vocabulary

Three measurements decided this, and each alone would be enough:

1. **Ambiguity.** 54 of 94 distinct ids are defined by more than one archived change, and `T1.1`
   is defined by twenty-two. A flat namespace cannot represent that; a scoped one still cannot for
   the nine capabilities that several changes touched.
2. **Alias contamination.** A constant carries aliases taken from task descriptions. Twenty-two
   descriptions attached to `T1.1` would let an incidental word ground a requirement to an arbitrary
   piece of history — the failure that was removed from `build_aliases` precisely because it silently
   outranked the ids an author wrote.
3. **The model cannot hold them.** `generate_promela` declares atoms only for the plan's tasks.
   A claim citing an archived id yields `undeclared variable`, `spin -a` fails, and the run reports
   "Model check error". Binding would convert an accurate diagnosis into an opaque failure.

The alternative I considered and rejected: declare archived tasks as **completed constants**
(`done_t2_6 = 1`, never active), so citations are checkable. It works mechanically and is wrong in
substance — an ordering against a completed task is tautologically satisfied, so the constraint would
enter the satisfied count as a proof. Tautologies counted as proofs are the defect this tool was just
cleaned of.

### D2 — Scope the scan to the requirement's capability

For a requirement in capability `C`, only archived changes whose `specs/C/` exists are scanned. That
is where the ids a `C` requirement cites can come from, and it keeps the scan proportional to a
capability's history rather than to the repository's.

**Cost:** nine capabilities here have more than one such change, so an id may have several owners.
That is not resolved by picking one; it is reported (D3).

### D3 — Report the owners, and report an id that does not distinguish

The finding names the owning changes, and when there are many it says how many. `T1.1 is defined by
22 archived changes` is the useful message — the author's citation does not identify a task, which is
different from a task being absent.

**Why not resolve ambiguities by recency?** A rule that picks the most recent owner would be right
most of the time and silently wrong the rest, and the caller cannot tell which case it is in. The
tool's value here is the fact, not a guess.

### D4 — The root is threaded, not rediscovered

The grounding check receives a `PlanIR` and a strictness, with no filesystem context. The root is
already resolved in `cmd_check` for input resolution, so it is passed down rather than re-derived
from `req.source.file`, which is relative to the change directory and differs between OpenSpec,
directory and single-file modes.

Five signatures change. That is the cost of no global, and the reason this is a change rather than a
patch.

## Risks

- **A stale archive names a change that no longer exists on a branch.** The finding is advisory
  guidance, so a wrong owner costs a wrong hint, not a wrong verdict.
- **A capability is renamed, so its history is not found.** The scan finds nothing and the diagnosis
  falls back to the current message, which is the status quo rather than a regression.
- **The scan is filesystem work on every check.** It is bounded by the number of archived changes
  that touched one capability, and it reads only the `tasks.md` id lines.

## Migration Plan

1. Thread the root (task 1.3). No behaviour change.
2. Add the scan and the owner map (tasks 1.1, 1.2). The corpus must not move: nothing grounds
   differently, only the message changes.
3. Extend the diagnosis (tasks 2.1, 2.2), then re-run the corpus and record it (task 3.3).

## Self-check

- Every requirement here is a forward ordering over tasks in this change's own `tasks.md`.
- The change does not depend on reading the archive to write its own specs, so its gate is
  meaningful before and after.
- The change removes no behaviour: grounding still never succeeds on a task this change does not
  define, so a plan that was checkable stays checkable and its verdicts are unchanged.

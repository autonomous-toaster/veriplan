## Context

See proposal.md for the measurements. This document records each decision that shapes the change, the
alternative that was rejected, and the reason — because most of these were decided by measurement
rather than preference, and the next reader deserves the number rather than the conclusion.

`check_convertibility` grounds each requirement against a `Signature` built from `plan.tasks`.
Translation happens later, in `verify`, and needs to know that a constraint refers to history so it
does not emit a formula for it.

## Goals / Non-Goals

**Goals:**

- A requirement that modifies a published requirement is groundable, so a delta can carry it.
- A constraint about completed work is reported as historical and is never a proof.
- Every fact the report states is one the archive can support, or is stated as missing.

**Non-Goals:**

- Re-checking the published corpus.
- Modelling history.
- Telling an added requirement from a modified one.

## Decisions

### D1 — The key is the requirement, not the task id

Per id, the archive is ambiguous: 54 of 94 ids are defined by more than one change and `T1.1` by
twenty-two. Per requirement, resolution is total: 111 of 111 published requirements resolve, 100
uniquely.

This is the hinge. Every other decision follows from it, and the alternative — a scoped id namespace
per capability — still leaves the nine capabilities that several changes touched ambiguous.

### D2 — Provenance is the latest archived change that contains the requirement and defines its ids

Both conditions are needed:

- *contains the requirement*, so a change that never touched it cannot own it;
- *defines every id the body cites*, so a change that carried the requirement forward without rewriting
  the body does not steal ownership from the change that wrote the body.

Among the 11 requirements with several candidates, the latest by archive order always wins, and it is
always the change that wrote the published body. Archive order is the `YYYY-MM-DD-` prefix, so the rule
depends on that prefix; if the archive ever stops being date-prefixed, resolution SHALL fail loudly
rather than silently take the first entry.

### D3 — Historical constraints carry no formula

A constraint whose arguments are all from the provenance change is about completed work. Encoding it
against the plan would produce a formula whose atoms are not this plan's tasks, and encoding the
archived tasks as completed constants would make the ordering tautologically true.

So the constraint carries no formula, is reported with the owning change, and is counted separately.
The precedent is `Unmodelled`, which the code already treats this way, with its own count and its own
reason.

### D4 — Historical is never blocking

Referring to completed work is not a fault in the plan; it is a fact about the requirement. Blocking
would restore the exact blocker this change removes, wearing a better message.

The protection against a plan passing on nothing is already in place: a run with no satisfied and no
violated constraint reports `valid: null`. A plan whose only constraint is historical therefore
reports no verdict, not a pass.

### D5 — A mixed constraint is reported, not failed

A constraint citing one live task and one historical one cannot be half-modelled: the historical atom
has no declaration. Classifying the whole constraint historical loses the half that could have been
checked, so the report names the foreign ids and the author can split the constraint.

**Rejected: failing the plan with "split this".** It is the harsher and arguably more honest option,
and it loses the change's purpose — the plan being carried is a reword, and a reword should not have
to split a published sentence to be accepted.

### D6 — Provenance is resolved once, above the grounding check

`check_grounding` receives a `PlanIR` and a strictness, with no filesystem context. Threading a root
into it would push I/O into a function that is currently pure and unit-testable.

Instead `cmd_check` resolves provenance once and passes a value down through the convertibility check.
Grounding stays pure; the archive is read in one place, at one level.

### D7 — Provenance is recorded on the requirement

`Requirement` already carries state that convertibility rewrites (`category` is set to
`PatternUngrounded` there), so `grounded_in: Option<String>` follows an existing precedent rather
than introducing a side channel that translation would have to look up.

### D8 — The total accounts for the fourth count

`total_constraints = satisfied + violated + unmodelled + unchecked` is stated in the published
`model-check` spec and holds today. Adding a fifth category without changing the invariant would make
the invariant false, so it SHALL become a sum over five.

### D9 — No caching

A full scan of 58 delta specs and 24 task lists is 6.5 ms. An index file would be a second source of
truth to keep in step with the archive, for a cost that does not need paying.

## Risks

- **The archive is not date-prefixed.** D2's tie-break breaks. Mitigated by failing loudly.
- **A requirement is renamed.** Resolution finds nothing and reports "no recorded provenance", which is
  the honest statement. Measured at 0 of 111 today.
- **A requirement exists in a delta whose tasks do not define its body's ids**, because the body was
  edited by hand between changes. Then no candidate matches and the requirement reports no provenance.
- **The published corpus is not re-checked by this change**, so a requirement whose provenance is
  resolvable but wrong would not be caught here. It is caught the first time a delta touches that
  requirement.
- **D5 loses a check.** Accepted explicitly: a mixed constraint is reported and not modelled, so the
  live half goes unchecked until the author splits it.

## Migration Plan

1. Resolve provenance and record it (phase 1). No behaviour change: nothing reads it yet.
2. Ground against the provenance tasks and classify (phase 2). Plans with historical citations start
   passing convertibility where they failed.
3. Report the new facts (phase 3), then re-run the corpus and record what moved (phase 4).

## Self-check

- Every requirement here is a forward ordering over tasks in this change's own `tasks.md`.
- The change does not need provenance to write its own specs, so its gate is meaningful before the fix
  as well as after.
- The change makes a class of plans checkable that was not: the corpus re-run in phase 4 is what proves
  it rather than asserts it.

## Why

When a requirement is not a verifiable temporal constraint, veriplan currently emits a
single generic blocker:

> SHALL '...' does not match any temporal category
> Fix: Add a temporal keyword to the requirement statement

This is **under-diagnosed and actively misleading**. Empirically, the `NonFormalizable`
path catches at least three *different* problems, all of which get the identical generic
message:

| Requirement | Actual problem | Current (misleading) fix |
|---|---|---|
| "T1.1 SHALL be done quickly" | references a task but uses a vague adverb ("quickly") | "Add a temporal keyword" |
| "The system SHALL be robust" | no task reference, vague adjective ("robust") | "Add a temporal keyword" |
| "T1.1 SHALL be executed" | references a task but specifies **no constraint** (redundant with the task list) | "Add a temporal keyword" |

None of these is fixed by "add a temporal keyword." The first needs a *measurable
definition*; the second needs a *task reference or measurable criterion*; the third is
*redundant with tasks.md* and should be made a constraint or removed.

This maps directly to the MIL-STD-498 requirement-evaluation criterion **"Testable"**:
a requirement is testable if "an objective and feasible test can be designed to determine
whether each requirement has been met." Vagueness *is* non-testability. veriplan should
diagnose *why* a requirement is untestable and give a targeted, pedagogical fix that
teaches the author the verifiable form — instead of a one-size-fits-all nudge toward a
temporal keyword.

## What Changes

Add a **vague-requirement diagnosis** layer on the existing `NonFormalizable` path.
When a requirement reaches that path (i.e. it has no temporal keyword and no
`human review only` marker), veriplan analyzes *why* it is non-formalizable and emits a
**targeted message + fix** instead of the generic "does not match any temporal category."

The diagnosis distinguishes three cases, all of which **remain blockers** (a vague
requirement is not verifiable; this change does NOT weaken the verifier — it only improves
the feedback):

1. **BareCapability** — references a task but specifies no constraint.
   - Message: "references T1.1 but specifies no constraint; this is redundant with the task list."
   - Fix: "add a temporal relation to another task (e.g. 'T1.1 SHALL complete BEFORE T1.2 SHALL start'), or remove it if it merely re-states the task."

2. **VagueAction** — references a task and uses a vague adverb.
   - Message: "references T1.1 but '<word>' is vague and not objectively testable."
   - Fix: "define it measurably (e.g. 'within 200ms'), or add a temporal relation to another task."

3. **VagueQuality** — no task reference and uses a vague adjective.
   - Message: "no task reference and '<word>' is vague."
   - Fix: "reference a task with a temporal relation, or define '<word>' via a measurable criterion or standard. For a safety statement, express it as a constraint, e.g. 'T1.1 SHALL fail safe IF T1.2 SHALL fail' or '...THROUGHOUT ...'."

4. **Undiagnosed** — no task reference and no vague word: falls back to the existing generic blocker (unchanged).

**Critical safety boundary (empirically verified):** temporal keywords always take
priority in `classify()` before reaching `NonFormalizable`. So vague detection only ever
runs on requirements that are genuinely non-temporal. A requirement like
"T1.1 SHALL be done quickly BEFORE T1.2 SHALL start" is still classified as `SequentialOrder`
(verifiable) and is never touched by vague detection. This change cannot misfire on a
verifiable requirement.

## Capabilities

### New Capabilities
- `vague-requirement-diagnosis`: Diagnose *why* a non-formalizable requirement is not
  verifiable (bare capability, vague action, vague quality) and emit targeted, pedagogical
  fixes instead of the generic "add a temporal keyword" message.

### Modified Capabilities
<!-- none — this changes no existing spec-level requirement behavior; it only improves the
     diagnostic message/fix on the existing non-formalizable path. -->

## Impact

- `src/translator/mod.rs`: add a `diagnose_vague(statement, task_refs)` helper (or similar)
  invoked when classification reaches `NonFormalizable`.
- `src/checker/checks.rs`: use the diagnosis to fill the `fix`/`detail` on the
  `non_formalizable` blocker.
- `src/ir/mod.rs`: no `ConstraintCategory` change needed — the diagnosis is a *message-level*
  refinement; verdicts and categories are unchanged.
- A small curated vague-word list (adverbs + adjectives + comparatives), hardcoded like the
  existing temporal-keyword list (no new config surface).
- Tests: unit tests for the three diagnoses + the safety-boundary case
  (temporal keyword present → never diagnosed as vague).
- **No behavior change to verdicts.** Pure feedback-quality improvement.

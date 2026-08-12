## Context

The grounding check (integrated in the previous change) produces a false positive for MAY requirements. When a requirement uses "MAY" instead of "SHALL"/"MUST", the groundcontrol `extract_shall()` function doesn't find a SHALL/MUST sentence and falls back to the full body text. Without a temporal predicate keyword (BEFORE, AFTER, etc.), the RuleGrounder returns Ungroundable — even when the text contains valid task IDs.

This was discovered when running `veriplan check veriplan-plan-verifier` — the deprecated BFS fallback requirement ("T6.7 MAY provide...") was flagged as ungroundable despite containing a valid task ID.

## Goals / Non-Goals

**Goals:**

- Skip grounding for MAY requirements (informational, not verifiable)
- Improve error message when task IDs exist but no predicate keyword matches
- Update the grounding-check spec to reflect these changes

**Non-Goals:**

- Change groundcontrol's `extract_shall()` function — the fix is in veriplan's grounding module, not in groundcontrol
- Change the convertibility check's handling of MAY requirements — they're already handled correctly by T4.4 (classified as NonFormalizable, flagged as info)

## Decisions

### Decision: Skip MAY requirements in the grounding loop

Add `if req.strength == Rfc2119Strength::May { continue; }` in the grounding loop in `src/grounding/mod.rs`.

**Rationale:** MAY requirements are informational by definition — they describe optional behavior that isn't verified by model checking. The convertibility check already handles them correctly (T4.4 classifies them as NonFormalizable, flags as info). Grounding them produces a false positive blocker that confuses users.

### Decision: Improve error message for predicate-missing vs task-missing

The current error message is: "Ungroundable requirement '...' — no matching task or predicate found"

This is misleading when task IDs ARE present but no predicate keyword matches. The improved message distinguishes two cases:

- **No task IDs found**: "Ungroundable requirement '...' — no matching task or predicate found. Add a task ID reference (e.g., 'T5.1') or a known predicate keyword."
- **Task IDs found but no predicate keyword**: "Ungroundable requirement '...' — no matching predicate keyword found (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE). Add a temporal keyword to the requirement statement."

**Rationale:** The distinction helps users understand what's actually wrong. If task IDs are present, the fix is to add a temporal keyword. If no task IDs are present, the fix is to add both.

## Risks / Trade-offs

- **[Missed grounding for MAY with SHALL-like structure]** A MAY requirement that happens to contain a temporal keyword (e.g., "T6.7 MAY run BEFORE T6.8") would be skipped by grounding but could still be classified by T4.4. → Acceptable: MAY is informational, the classification is for reporting only.
- **[Error message coupling]** The improved error message checks whether any constant name appears in the text. This is a heuristic — it might incorrectly say "task IDs found" when the text contains a substring that happens to match a constant name. → Acceptable: the heuristic is the same one the grounder uses for argument extraction.

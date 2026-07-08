---
name: veriplan-spec
description: Write OpenSpec requirements that pass veriplan model checking on the first try. Use when creating or editing spec.md files in an OpenSpec change.
---

Write veriplan-compliant OpenSpec requirements.

veriplan checks that every SHALL requirement can be translated into a temporal constraint on tasks and verified by model checking. Requirements that describe code properties ("the function SHALL return correct results") will be rejected — they must describe task ordering.

## Rules (violating any = veriplan blocker)

1. **Every SHALL MUST reference at least one task by N.M ID**
   - GOOD: `T2.1 SHALL complete BEFORE T3.1`
   - BAD: `The system SHALL be robust` (no task ID)

2. **Every SHALL MUST use exactly ONE temporal keyword**
   - Keywords: `BEFORE`, `AFTER`, `CONCURRENTLY`, `IF...THEN`, `ALWAYS`, `AT MOST ONE`
   - GOOD: `T2.1 SHALL complete BEFORE T2.4`
   - BAD: `T2.1 SHALL complete` (no temporal keyword)

3. **Task IDs in spec MUST match task IDs in tasks.md**
   - Read tasks.md first. Use the exact IDs from there.
   - If tasks.md has `1.1`, `2.1`, `2.2` — use `T1.1`, `T2.1`, `T2.2` in the spec.

4. **`[concurrent]` MUST be in a `## Phase` heading**
   - GOOD: `## Phase 2: Harnesses [concurrent]`
   - BAD: `## 2. Harnesses [concurrent]` (doesn't start with "Phase")

5. **Every scenario MUST have WHEN + THEN with RFC 2119 keyword**
   - GOOD: `- **WHEN** T2.1 runs\n- **THEN** the harness SHALL pass`
   - BAD: Missing WHEN or THEN, or THEN without SHALL/MUST

6. **Requirements MUST describe task ordering, not code properties**
   - GOOD: `T2.1 SHALL complete BEFORE T2.4` (task A before task B)
   - BAD: `collect_atoms() SHALL find all atoms` (code property, not task ordering)

7. **Avoid comma-separated task lists in SHALL statements**
   - The grounder has low confidence with `T2.1, T2.2, and T2.3 SHALL run CONCURRENTLY`
   - Instead, ensure tasks are in a `[concurrent]` phase and reference one task:
   - GOOD: `T2.1 SHALL run CONCURRENTLY with T2.2`

## Template

```markdown
## Task Reference

| Task ID | Description |
|---------|-------------|
| T<phase>.<seq> | <description from tasks.md> |

## ADDED Requirements

### Requirement: <name describing the ordering constraint>

T<id> SHALL <action> <TEMPORAL_KEYWORD> T<id> SHALL <action>.

#### Scenario: <scenario name>

- **WHEN** <condition referencing a task>
- **THEN** <expected outcome with SHALL/MUST>
```

## Workflow

1. Read `tasks.md` first — get the exact task IDs and phase structure
2. For each requirement, pick ONE temporal keyword from the list
3. Write the SHALL statement referencing task IDs
4. Add at least one scenario with WHEN + THEN
5. Run `veriplan check` to validate
6. If it fails, fix the blockers (they tell you exactly what's wrong)

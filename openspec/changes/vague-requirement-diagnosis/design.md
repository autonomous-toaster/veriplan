## Context

The `NonFormalizable` path in `src/translator/mod.rs` `classify()` is reached only when a
requirement has **no temporal keyword and no `human review only` marker**. `check_classifiability`
in `src/checker/checks.rs` turns that into a blocker with the generic message:
"does not match any temporal category — Fix: Add a temporal keyword."

Empirically this path catches three distinct problems (bare capability, vague action, vague
quality) that all get the same misleading fix. veriplan already computes task references
(`extract_task_refs_bare`), so the task-reference signal is available at the point of diagnosis.

The classification priority in `classify()` is fixed: temporal categories first, then
informational, then `NonFormalizable`. This provides a hard safety boundary for any new
diagnostic — it can never run on a verifiable requirement.

## Goals / Non-Goals

**Goals:**
- Diagnose *why* a non-formalizable requirement is not verifiable, into one of:
  BareCapability, VagueAction, VagueQuality (or Undiagnosed fallback).
- Emit a targeted, pedagogical fix that teaches the verifiable form.
- Keep verdicts unchanged (all vague requirements remain blockers) — this is a
  feedback-quality improvement, not a verification-weakening.
- Keep the change surgical: message/fix refinement only, on the existing non-formalizable path.

**Non-Goals:**
- No new `ConstraintCategory` variants.
- No reclassification of vague requirements to informational/non-blocking.
- No configurable word list / new CLI surface.
- No trend tracking or blast-radius analysis (out of scope).
- No changes to the `human review only` / informational marker behavior.

## Decisions

**D1 — Diagnostic lives in the translator, consumed by the checker.**
A `diagnose_vague(statement: &str, task_refs: &[String]) -> Option<VagueDiagnosis>` helper in
`src/translator/mod.rs`, called by `check_classifiability` only when `classify()` returned
`NonFormalizable`. This keeps `classify()`'s category contract unchanged (still returns
`NonFormalizable`), and the checker uses the diagnosis to populate `detail` + `fix`.
*Alternative considered:* new enum variants (`VagueAdverb`, etc.). Rejected — that ripples
through bfs/distribution and is higher-risk for no verification benefit.

**D2 — Three diagnoses, ordered by signal strength.**
For a non-formalizable requirement:
1. **BareCapability** if it has a task reference and no vague word.
2. **VagueAction** if it has a task reference and a vague adverb.
3. **VagueQuality** if it has no task reference and a vague adjective.
4. **Undiagnosed** otherwise (fall back to the existing generic blocker).
Task-reference check is the strongest grounding signal; vague-word presence splits
"constraint-shaped but vague" from "assertion but no constraint."

**D3 — Vague-word list is small, curated, hardcoded.**
Adverbs: `quickly, fast, soon, correctly, properly, efficiently, reliably, safely,
adequately, promptly, easily, consistently`. Adjectives: `robust, clean, good, stable,
user-friendly, easy, fast, minimal, optimal, responsive, scalable, performant, reliable,
safe`. Comparatives (no basis): `better, worse, best, as fast as possible, as soon as
possible`. Mirrors the existing curated temporal-keyword approach. Configurability is a
non-goal (D2 of non-goals).
*Risk noted:* "safe"/"reliably" can appear in legitimate safety statements. This is
mitigated by the safety boundary — legitimate safety *constraints* ("T1.1 SHALL fail safe
IF T1.2 SHALL fail", "...THROUGHOUT ...") classify as temporal and are never diagnosed.
Only *bare* non-temporal safety statements (genuinely vague) reach this path.

**D4 — Pedagogical fixes teach the verifiable form.**
Each diagnosis's fix shows a concrete target: "define '<word>' measurably (e.g. 'within
200ms')", "express a safety statement as 'T1.1 SHALL fail safe IF T1.2 SHALL fail' or
'...THROUGHOUT ...'". This turns the generic "add a temporal keyword" into actionable,
form-teaching guidance.

## Risks / Trade-offs

- **False-positive on the vague-word list:** confined to the non-formalizable bucket by the
  safety boundary; within that bucket, a vague diagnosis is strictly better than the current
  generic message. Low risk.
- **Verdicts unchanged:** vague requirements still block. This avoids weakening the verifier
  but means a genuinely harmless-but-redundant bare capability still blocks. Accepted —
  aligns with the "unambiguous spec" philosophy and keeps the change purely diagnostic.
- **Hardcoded word list:** not configurable; may miss some words or over-match others.
  Acceptable for a first cut; easy follow-up to make it configurable if demand emerges.

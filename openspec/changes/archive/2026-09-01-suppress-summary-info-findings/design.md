## Context

See proposal.md for motivation. veriplan currently emits five plan-level aggregate diagnostics as `info`-severity findings (`task_count`, `scenario_count`, `classification_summary`, `constraint_diversity`, `grounding_summary`). Because those check names are absent from the `Kind` enum, `kind_of()` maps them all to `ProseOther`, so they collapse into a meaningless `N× prose_other` row in default output and five noise lines under `--verbose`.

## Goals / Non-Goals

**Goals:**
- Keep default output to genuine findings (blockers, warnings, violations, and advisories that carry a suggestion).
- Remove the five aggregate summaries from the `findings[]` set consistently across JSON and human output.
- Preserve the plan-level counts under `--verbose`.

**Non-Goals:**
- Changing model-checking, grounding, or classification logic.
- Removing genuine advisory findings with a fix suggestion (`low_diversity`, `may_requirement`, `informational_requirement`).

## Decisions

### D1 — Stop emitting the aggregates as findings

Remove the `info.push(make_item("info", ...))` for `task_count`, `scenario_count`, `classification_summary`, `constraint_diversity`, and `grounding_summary` from the checks. They are internal diagnostics, not findings.

**Why remove from the source rather than filter in the formatter?** veriplan guarantees JSON and human `findings[]` describe the same set (design D5). Filtering only in the human formatter would break that invariant (and its test). Removing from the source keeps both formats consistent and honest.

### D2 — Surface counts in the verbose plan summary

The annotator's `verbose_section` already prints `Tasks`/`Requirements`/`Phases` from `plan`. Extend it to also print `Scenarios`. The richer diagnostics that are dropped (`classification_summary`, `constraint_diversity`, grounding-complete) are recomputable and not part of the actionable contract.

### D3 — Keep genuine advisories

`low_diversity` (carries a fix suggestion), `may_requirement`, and `informational_requirement` have dedicated `Kind` variants (not `ProseOther`) and remain in the findings stream unchanged. They are not affected by this change.

## Risks / Trade-offs

- **Dropping verbose diagnostics** — the `constraint_diversity` and formalizable/non-formalizable counts no longer appear anywhere. Mitigation: these are recomputable from the plan and were judged non-actionable; the user explicitly flagged them as noise. The task/phase/scenario counts remain in the verbose summary.
- **JSON consumers** — a consumer reading `findings[]` will no longer see the five counts. Mitigation: they were diagnostics, not contract; this is the intended behavior change.

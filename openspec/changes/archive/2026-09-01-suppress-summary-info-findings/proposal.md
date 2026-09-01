## Why

`veriplan check` surfaces plan-level aggregate diagnostics — task/phase counts, scenario counts, classification (`N formalizable, N non-formalizable`), constraint distribution, and a "grounding check complete" note — as `info`-severity *findings*. These check names (`task_count`, `scenario_count`, `classification_summary`, `constraint_diversity`, `grounding_summary`) are absent from veriplan's `Kind` enum, so they all collapse into `prose_other`. In default output they appear as a meaningless `N× prose_other` row; under `--verbose` they expand into five lines that carry no actionable information.

Default output should only show *findings* (blockers, warnings, violations, and genuine advisories with a suggestion) — not internal accounting.

## What Changes

- Stop emitting five plan-level aggregate diagnostics as findings:
  - `task_count` ("Found N tasks across M phases")
  - `scenario_count` ("Found N scenarios across all spec files")
  - `classification_summary` ("N formalizable, N non-formalizable requirements")
  - `constraint_diversity` ("Constraint distribution: …")
  - `grounding_summary` ("Grounding check complete: N requirements checked")
- Keep the plan-level counts available under `--verbose` by extending the existing plan summary (`verbose_section`) with the scenario count.
- Leave genuine advisory findings intact: `low_diversity` (has a fix suggestion), `may_requirement`, `informational_requirement` (they have dedicated kinds, not `prose_other`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `output-contract`: the `findings[]` set SHALL NOT include plan-level aggregate diagnostics (task/phase count, scenario count, classification summary, constraint distribution, grounding summary); those are internal diagnostics surfaced only in the `--verbose` plan summary. The JSON and human `findings[]` SHALL stay consistent (both drop them).

## Impact

- `src/checker/checks.rs` — drop `task_count`, `scenario_count`, `constraint_diversity` info emissions.
- `src/checker/checks/classifiability.rs` — drop `classification_summary` info emission.
- `src/grounding/mod.rs` — drop `grounding_summary` info emission.
- `src/annotator/mod.rs` — `verbose_section` also prints the scenario count.
- No change to the model-checking or grounding logic; only to which synthesized summaries are reported as findings.

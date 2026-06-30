Task Reference

| T ID | Description |
|------|-------------|
| T2.1 | Add ConstraintCategory::PatternUngrounded variant |
| T2.2 | Split check_classifiability into pattern detection and grounding checks |
| T2.3 | Change MAY requirements from silent drop to INFO items |
| T3.1 | Add StrictnessProfile enum (Strict, Moderate, Lax) |
| T3.2 | Add --strict/--moderate/--lax CLI flags |
| T3.3 | Implement severity mapping per StrictnessProfile in checker |
| T3.4 | Make "no tasks"/"no requirements" severity depend on input mode |

### Requirement: Three-tier classification separates pattern from grounding

T2.2 SHALL split the classifiability check into pattern detection and grounding checks BEFORE T2.1 adds the `PatternUngrounded` variant.
A requirement whose `classify()` returns a temporal category but whose `extract_task_refs()` returns empty SHALL be classified as `PatternUngrounded` BEFORE the check report is emitted.
A requirement with no temporal keyword SHALL remain `NonFormalizable`.

### Requirement: PatternUngrounded severity depends on strictness profile

T3.1 SHALL define the `StrictnessProfile` enum BEFORE T3.3 maps `PatternUngrounded` severity per profile.
In `Strict` mode, `PatternUngrounded` SHALL be BLOCKER with detail "formalizable pattern detected but no task references — add task IDs for model verification".
In `Moderate` mode, `PatternUngrounded` SHALL be WARNING with detail "formalizable pattern detected but not model-verifiable — add task IDs for full verification".
In `Lax` mode, `PatternUngrounded` SHALL be INFO with detail "formalizable pattern detected".

### Requirement: MAY requirements are reported as INFO

T2.3 SHALL emit MAY-strength requirements as INFO items BEFORE the classifiability check runs.
MAY items SHALL have check name "may_requirement", element "Requirement '{id}'", and detail "MAY requirement — not model-verified, informational per RFC 2119".
MAY items SHALL NOT be silently dropped from the report.

### Requirement: Strictness flags are mutually exclusive

T3.1 SHALL define the `StrictnessProfile` enum BEFORE T3.2 adds the `--strict`, `--moderate`, and `--lax` flags.
`--strict` SHALL be the default.
Specifying more than one strictness flag SHALL cause veriplan to exit with an error.

### Requirement: Context-aware "no content" severity

T3.4 SHALL make "no tasks found" and "no requirements found" severity depend on input source mode BEFORE emitting the check item.
In OpenSpec mode, both SHALL be BLOCKER.
In single-file, stdin, or loose-directory mode, both SHALL be INFO.
The severity SHALL NOT depend on the `--strict`/`--moderate`/`--lax` flag.

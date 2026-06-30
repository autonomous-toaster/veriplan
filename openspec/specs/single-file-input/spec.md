Task Reference

| T ID | Description |
|------|-------------|
| T1.2 | Implement single-file input detection and parsing |
| T1.3 | Implement stdin input detection and parsing |
| T4.1 | Add parse_content() function that tries both parsers on any content |
| T4.2 | Make PlanIR construction tolerant of empty tasks or empty requirements |

### Requirement: parse_content works on any markdown content

T4.1 SHALL implement `parse_content(source: &str, filename: &str)` that applies `parse_tasks()` and `parse_spec()` to the same content BEFORE building PlanIR.
If `parse_tasks()` returns tasks, they SHALL be included in the PlanIR.
If `parse_spec()` returns requirements, they SHALL be included in the PlanIR.
If both return empty results, `parse_content()` SHALL return an error "no verifiable content found".

### Requirement: Single-file mode parses content, not filename

T1.2 SHALL NOT use filename heuristics to decide parsing strategy AFTER reading the file content.
The same `parse_content()` function SHALL be used for both file and stdin input.
The filename SHALL be used only for source locations in error messages and LSP diagnostics.

### Requirement: PlanIR tolerates partial input

T4.2 SHALL allow PlanIR construction with an empty `tasks` vector AFTER T4.1 parses content.
PlanIR with empty `tasks` SHALL trigger "no tasks found" as INFO (single-file/stdin) or BLOCKER (OpenSpec mode) BEFORE the checker runs.
PlanIR with empty `requirements` SHALL trigger "no requirements found" as INFO (single-file/stdin) or BLOCKER (OpenSpec mode) BEFORE the checker runs.

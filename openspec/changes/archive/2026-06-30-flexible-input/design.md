# Design: Flexible Input

## Architecture

```
CLI args ──▶ InputResolver ──▶ parse_content() ──▶ PlanIR ──▶ Checker(Strictness)
                  │                                                     │
                  │  InputResolver detects:                             │  Checker
                  │  ┌────────────────┐                                 │  produces
                  │  │ OpenSpec dir    │──▶ locate_change()             │  CheckItems
                  │  │ (current)      │                                 │  with
                  │  ├────────────────┤                                 │  severity
                  │  │ Loose dir      │──▶ parse tasks.md/specs/        │  controlled
                  │  │ (has some)    │     if present                  │  by profile
                  │  ├────────────────┤                                 │
                  │  │ Single file    │──▶ parse_content(file)           │
                  │  │ (.md)          │     auto-detect tasks+reqs      │
                  │  ├────────────────┤                                 │
                  │  │ Stdin          │──▶ parse_content(stdin)         │
                  │  │ (- or --stdin) │     auto-detect tasks+reqs      │
                  │  └────────────────┘                                 │
                  │                                                     │
                  │  If nothing to verify → ERROR, not silent          │
                  └─────────────────────────────────────────────────────┘
```

## Three-Tier Classification

Currently, `classify()` returns `NonFormalizable` for anything without a temporal keyword. This conflates two distinct cases:

| Pattern | Temporal keyword? | Task IDs? | Current | Proposed |
|---------|-------------------|-----------|---------|----------|
| "SHALL be fast" | No | No | NonFormalizable | NonFormalizable (blocker) |
| "X SHALL complete before Y" | Yes | No | SequentialOrder (but check_task_ids blocks) | PatternUngrounded (severity varies by profile) |
| "T1.2 SHALL complete before T3.1" | Yes | Yes | SequentialOrder (model-verifiable) | FullyFormalizable (all clear) |

New category: `PatternUngrounded` — temporal pattern detected, but no task references to ground it in the state machine. Severity depends on strictness profile.

## Strictness Profiles

| Finding | strict (default) | moderate | lax |
|---------|-----------------|----------|-----|
| NonFormalizable (no pattern) | BLOCKER | BLOCKER | WARNING |
| PatternUngrounded (pattern, no task IDs) | BLOCKER | WARNING | INFO |
| Undefined task reference (T9.9) | BLOCKER | BLOCKER | WARNING |
| MAY requirement | INFO | INFO | INFO |
| Uncovered task | WARNING | WARNING | INFO |
| Missing scenario | WARNING | INFO | INFO |
| No tasks found (single-file mode) | INFO | INFO | INFO |
| No requirements found (single-file mode) | INFO | INFO | INFO |
| No tasks found (OpenSpec mode) | BLOCKER | BLOCKER | WARNING |
| No requirements found (OpenSpec mode) | BLOCKER | BLOCKER | WARNING |
| Duplicate task ID | BLOCKER | BLOCKER | BLOCKER |

Key principle: **nothing is silently dropped**. Every finding is reported. The profile only controls severity.

## Input Resolution

```rust
enum InputSource {
    OpenSpec { change_dir: PathBuf, change_name: String },
    Directory { path: PathBuf },
    SingleFile { path: PathBuf, content: String },
    Stdin { content: String, label: String },
}
```

Detection priority:

1. Argument is a directory with `openspec/changes/` → OpenSpec mode
2. Argument is a directory with `tasks.md` or `specs/` → Directory mode
3. Argument is a `.md` file → Single-file mode
4. Argument is `-` or `--stdin` flag → Stdin mode
5. Argument is a string (no path separators) → OpenSpec change name lookup
6. No argument, CWD has `openspec/changes/` → OpenSpec auto-detect
7. No argument, CWD has `tasks.md` or `specs/` → Directory mode on CWD
8. None of the above → clear error message

## Parse Strategy

The existing `parse_tasks()` and `parse_spec()` functions already accept `(&str, &Path)` — raw content. The new `parse_content()` function:

1. Try `parse_tasks()` on the full content
2. Try `parse_spec()` on the full content (with a synthetic capability name)
3. Merge results into PlanIR (either or both may be empty)
4. If both are empty → "nothing to verify" error

No filename-based heuristics. The parser looks at content structure, not filenames.

## Default Strictness by Mode

OpenSpec mode defaults to `--strict`. All other modes also default to `--strict`. The user must explicitly opt into `--moderate` or `--lax`. This ensures the tool is always strict by default — the user must consciously choose to relax.

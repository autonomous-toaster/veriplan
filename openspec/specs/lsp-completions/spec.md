# lsp-completions

## Purpose

Provides task ID and temporal keyword completion suggestions in spec files, triggered by typing `T`/`t` or after RFC 2119 keywords like SHALL/MUST.

## Task Reference

| T ID | Description |
|------|-------------|
| T4.1 | Implement completion handler for task ID suggestions |
| T4.2 | Build completion items with task ID format |
| T4.3 | Implement temporal keyword completions |
| T4.4 | Register trigger characters in server capabilities |

## Requirements

### Requirement: Task ID completions

T4.1 SHALL ALWAYS provide `textDocument/completion` returning task ID suggestions when triggered by the character `T` or `t` in a spec file that belongs to an OpenSpec change.

T4.2 SHALL ALWAYS format each completion item with:

- label: "T3.2 — Auto-detect changes"
- kind: CompletionItemKind::Variable
- detail: "Phase: Detect Changes"
- insertText: "3.2" (the numeric ID without the T prefix)

#### Scenario: Completion triggers after T

- **WHEN** T4.1 detects that `T` is typed in a spec file at the start of a requirement paragraph
- **THEN** T4.1 SHALL return all task IDs from the containing change with their descriptions

#### Scenario: Completion returns empty for files outside any change

- **WHEN** T4.1 receives a completion request for a file not under any OpenSpec change
- **THEN** T4.1 SHALL return an empty completion list

### Requirement: Temporal keyword completions

T4.3 SHALL ALWAYS provide completions for temporal keywords when triggered in a spec file context where SHALL/MUST has been typed.

Each completion item SHALL contain:

- label: "BEFORE"
- kind: CompletionItemKind::Keyword
- detail: "Sequential — T<N> SHALL complete BEFORE T<N>"
- insertText: "BEFORE T"

The keywords SHALL be: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, AT MOST ONE.

#### Scenario: Keyword completion after SHALL

- **WHEN** T4.3 receives a completion request after user types `SHALL` in a spec file
- **THEN** T4.3 SHALL return the 6 temporal keywords as completion suggestions

### Requirement: Completion trigger characters

T4.4 SHALL ALWAYS register `["T", "t"]` as completion trigger characters in the server capabilities returned during `initialize`.

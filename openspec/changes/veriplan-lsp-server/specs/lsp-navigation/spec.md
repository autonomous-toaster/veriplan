Task Reference

| T ID | Description |
|------|-------------|
| T5.1 | Implement go-to-definition handler |
| T5.2 | Implement SourceLocation to LSP Location conversion |
| T5.3 | Implement hover handler |

## ADDED Requirements

### Requirement: Go-to-definition for task references

T5.1 SHALL ALWAYS implement `textDocument/definition` that resolves a task reference (e.g., `T3.2`) at a cursor position in a spec file to its location in `tasks.md` within the same change directory.

The response SHALL be a `Location` with:
- uri: file URI of `tasks.md` in the change directory
- range: the SourceLocation of the referenced task's list item (start_line - 1 for LSP 0-based, character 0)

#### Scenario: Go-to-definition on valid task reference

- **WHEN** T5.1 is invoked with cursor on `T3.2` in a spec file
- **THEN** T5.1 SHALL return a Location pointing to the list item for task 3.2 in tasks.md

#### Scenario: Go-to-definition on non-existent task reference

- **WHEN** T5.1 is invoked with cursor on `T9.9` which does not exist in the plan
- **THEN** T5.1 SHALL return None

### Requirement: Hover information for task references

T5.3 SHALL ALWAYS implement `textDocument/hover` that shows a MarkupContent tooltip for a task reference in a spec file.

The hover content SHALL be a markdown string:
```
**T3.2** — Auto-detect changes

*Phase:* Detect Changes
```

#### Scenario: Hover on valid task reference

- **WHEN** T5.3 is invoked on `T3.2` in a spec file
- **THEN** T5.3 SHALL return markdown showing the task ID, description, and phase

#### Scenario: Hover on non-task text

- **WHEN** T5.3 is invoked on text not matching a task reference pattern
- **THEN** T5.3 SHALL return None

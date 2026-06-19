Task Reference

| T ID | Description |
|------|-------------|
| T7.1 | Implement code action handler surfacing CheckItem.fix |
| T7.2 | Store CheckItem.fix in Diagnostic.data during diagnostics |
| T7.3 | Build WorkspaceEdit for each code action |
| T7.4 | Skip diagnostics without fix suggestions |

## ADDED Requirements

### Requirement: Code actions from diagnostic fixes

T7.1 SHALL ALWAYS implement `textDocument/codeAction` that surfaces quick fixes for diagnostics that have a CheckItem.fix value. Each fix SHALL be returned as a `CodeAction` with:
- title: the fix text from the diagnostic
- kind: CodeActionKind::QUICKFIX
- diagnostics: [the originating Diagnostic]
- edit: a WorkspaceEdit containing a TextEdit replacing the problematic range with the corrected text

T7.4 SHALL ALWAYS skip diagnostics where CheckItem.fix is None — only diagnostics WITH a suggested fix SHALL produce code actions.

#### Scenario: Quick fix for bad task reference

- **WHEN** T7.1 is invoked on a range with a "bad_task_reference" diagnostic and T7.2 stored the fix in Diagnostic.data
- **THEN** T7.1 SHALL return a CodeAction with title like "Change to: T3.2 (Auto-detect changes)"

#### Scenario: No code action for unfixable issues

- **WHEN** T7.1 is invoked on a range with a diagnostic that has no CheckItem.fix (fix is None)
- **THEN** T7.4 SHALL ensure the diagnostic produces no CodeAction

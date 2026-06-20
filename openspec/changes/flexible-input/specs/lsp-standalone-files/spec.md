Task Reference

| T ID | Description |
|------|-------------|
| T5.1 | Update LSP ChangeStore to handle single-file resolution |
| T5.2 | Add parse_content() support to LSP diagnostics |

### Requirement: LSP resolves single files outside OpenSpec changes

T5.1 SHALL update `ChangeStore::resolve_change()` to attempt single-file parsing BEFORE returning "file not in any change".
When a file is not in any OpenSpec change directory, T5.1 SHALL parse it as a standalone spec or tasks file.
If parsing succeeds, LSP diagnostics SHALL be published for that file.
If parsing fails, empty diagnostics SHALL be published to clear stale markers.

### Requirement: LSP diagnostics work for standalone files

T5.2 SHALL use `parse_content()` for standalone files AFTER T5.1 resolves them.
Diagnostics for standalone files SHALL include convertibility warnings and errors.
Standalone files SHALL NOT trigger model checking (no SPIN).

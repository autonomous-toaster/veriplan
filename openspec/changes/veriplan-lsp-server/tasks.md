## 1. Scaffold and Dependencies

- [x] 1.1 Add lsp-server, lsp-types dependencies to Cargo.toml
- [x] 1.2 Create `src/lsp/mod.rs` module structure with submodules: transport, state, diagnostics, completions, navigation, symbols, code_actions
- [x] 1.3 Add `Lsp` variant to `Commands` enum in main.rs and dispatch to `run_lsp()`
- [x] 1.4 Implement `run_lsp()` handler in main.rs

## 2. ChangeStore (Workspace State)

- [x] 2.1 Implement `ChangeStore` with `HashMap<String, PlanIR>` by change name, `HashMap<PathBuf, String>` file-to-change reverse index
- [x] 2.2 Implement `resolve_change(path: &Path) -> Option<String>` walking parent dirs for `openspec/changes/<name>/` pattern
- [x] 2.3 Implement `refresh(change: &str) -> Result<Vec<Diagnostic>, String>` that re-parses the change directory and runs convertibility check
- [x] 2.4 Implement `has_change(path: &Path) -> bool` and `get_plan(path: &Path) -> Option<&PlanIR>`
- [x] 2.5 Protect all methods behind `Arc<RwLock<ChangeStore>>` with read/write method split

## 3. Diagnostics (didSave → publishDiagnostics)

- [x] 3.1 Implement `did_save()` handler that calls `store.refresh()`, maps CheckItems to LSP Diagnostics
- [x] 3.2 Map CheckItem severity ("blocker"/"warning"/"info") → DiagnosticSeverity (Error/Warning/Information)
- [x] 3.3 Parse CheckItem.location ("file:line") → LSP Range with 0-based line conversion
- [x] 3.4 Publish diagnostics for ALL files in the affected change (not just the saved file)
- [x] 3.5 Handle files outside any change: publish empty diagnostics for that file only

## 4. Completions

- [x] 4.1 Implement `completion()` handler for task ID suggestions triggered by `T`/`t`
- [x] 4.2 Build completion items with label "T1.3 — description", detail "Phase: name", insertText "1.3"
- [x] 4.3 Implement temporal keyword completions: BEFORE, CONCURRENTLY, AFTER, IF...THEN, ALWAYS, AT MOST ONE
- [x] 4.4 Register `["T", "t"]` as trigger characters in server capabilities

## 5. Navigation (Go-to-definition and Hover)

- [x] 5.1 Implement `goto_definition()` handler that resolves `T<N>` at cursor → Location in tasks.md
- [x] 5.2 Implement SourceLocation → LSP Location conversion (1-based to 0-based line)
- [x] 5.3 Implement `hover()` handler returning task description and phase as MarkupContent
- [x] 5.4 Handle non-existent task refs: return None instead of crashing

## 6. Document Symbols

- [x] 6.1 Implement `document_symbols()` for tasks.md: phases as Namespace, tasks as Function children
- [x] 6.2 Implement document_symbols for spec.md: requirements as Interface, scenarios as Event children
- [x] 6.3 Show temporal category in detail field for spec.md symbols

## 7. Code Actions

- [x] 7.1 Implement `code_action()` handler surfacing CheckItem.fix as CodeAction with QuickFix kind
- [x] 7.2 Store CheckItem.fix in Diagnostic.data during diagnostics publishing
- [x] 7.3 Generate WorkspaceEdit with TextEdit for each code action
- [x] 7.4 Skip diagnostics without fix suggestions (no code action for unfixable issues)

## 8. Edge Cases and Polish

- [x] 8.1 Handle project root without openspec/ directory (all queries return empty)
- [x] 8.2 Handle tasks.md or specs/ missing from a change directory
- [x] 8.3 Log startup errors to stderr (visible in editor LSP logs)
- [x] 8.4 Test on veriplan's own change (dogfood) — veriplan lsp starts, logs to stderr, and exits gracefully when no client connects

## 9. pi-lens Integration and Docs

- [x] 9.1 Document the `.pi-lens/lsp.json` config snippet in README
- [x] 9.2 Add `--stdio` flag documentation to `veriplan lsp --help`
- [x] 9.3 Add example pi-lens config to the project's own `.pi-lens/lsp.json` for dogfooding

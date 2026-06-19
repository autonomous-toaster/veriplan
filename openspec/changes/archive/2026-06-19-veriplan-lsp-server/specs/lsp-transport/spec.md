Task Reference

| T ID | Description |
|------|-------------|
| T1.4 | Implement run_lsp with tokio runtime |

## ADDED Requirements

### Requirement: LSP stdio transport

T1.4 SHALL ALWAYS implement the Language Server Protocol over stdin/stdout using tower-lsp when invoked via `veriplan lsp --stdio`.

The server SHALL accept `InitializeParams` with TextDocumentSyncKind::Incremental capability and respond with server capabilities advertising:
- textDocumentSync: Save (not incremental — full content on save)
- completionProvider with trigger characters ["T", "t"]
- definitionProvider: true
- hoverProvider: true
- documentSymbolProvider: true
- codeActionProvider: true

#### Scenario: Server starts and responds to initialize

- **WHEN** T1.4 receives an `initialize` request with workspace root pointing to a veriplan project
- **THEN** T1.4 SHALL respond with InitializeResult containing the capabilities listed above

#### Scenario: Server handles shutdown gracefully

- **WHEN** T1.4 receives a `shutdown` request followed by `exit`
- **THEN** T1.4 SHALL flush pending diagnostics, release all state, and exit with code 0

### Requirement: File-to-change resolution

T2.2 SHALL ALWAYS resolve any opened document to its containing OpenSpec change directory by walking parent directories until it finds a path matching the pattern `openspec/changes/<change_name>/`. The resolved change name SHALL scope all subsequent LSP queries.

#### Scenario: File inside a change resolves correctly

- **WHEN** T2.2 receives a file path under `openspec/changes/my-change/specs/capability/spec.md`
- **THEN** T2.2 SHALL resolve the change name to `my-change`

#### Scenario: File outside any change returns empty results

- **WHEN** T2.2 receives a file not under any `openspec/changes/<name>/` directory
- **THEN** T2.2 SHALL return None (no diagnostics, no completions, no navigation)

### Requirement: Workspace state management

T2.1 AND T2.2 SHALL ALWAYS maintain an in-memory ChangeStore holding parsed PlanIR for each known change directory, a file-to-change reverse index, and cached convertibility reports. The store SHALL be protected by `Arc<RwLock<>>`.

#### Scenario: State is updated on file save

- **WHEN** T2.2 receives a `didSave` for a spec file under a known change
- **THEN** T2.1 SHALL re-parse the change directory and T3.1 SHALL run the convertibility check, then T3.4 SHALL publish new diagnostics

### Requirement: No model checking in LSP

T1.4 SHALL ALWAYS skip SPIN model checking for all LSP events. SPIN verification SHALL remain exclusive to the `veriplan check` CLI command.

#### Scenario: didSave does not trigger SPIN

- **WHEN** T3.1 receives a `didSave` notification
- **THEN** T3.1 SHALL run only `checker::check_convertibility()` — no `spin -a`, no `gcc`, no `pan` invocation

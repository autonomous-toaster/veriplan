## Why

Veriplan currently provides feedback only on explicit CLI invocation (`veriplan check`). Authors of OpenSpec plans get no real-time guidance as they write, and AI agents (like pi) parse free-text CLI output rather than interacting through a structured protocol. Adding an LSP subcommand closes both gaps: instant diagnostics, completions, and navigation in editors, and a structured JSON-RPC interface for agents.

## What Changes

- **New `veriplan lsp --stdio` subcommand** — runs veriplan as a Language Server Protocol daemon over stdio, managed by pi-lens or any LSP client
- **Real-time diagnostics** — convertibility check results (blockers, warnings) published as LSP diagnostics on file save, with the same severity mapping as `veriplan check`
- **Task ID completions** — suggest task IDs with descriptions when typing `T` in spec files
- **Temporal keyword completions** — suggest `BEFORE`, `CONCURRENTLY`, `AFTER`, etc. after `SHALL`
- **Go-to-definition** — jump from `T3.2` in a spec file to its task definition in `tasks.md`
- **Hover information** — show task description and phase when hovering over a task reference
- **Document symbols** — hierarchical outline of phases, tasks, requirements, and scenarios
- **Code actions** — quick fixes for diagnosable issues (bad task refs, non-formalizable constraints)
- **No model checking** in LSP — SPIN is too expensive for real-time; offered as workspace command only
- **No new CLI flags** — reuse `--stdio` convention standard across LSP servers
- **pi-lens integration** — `.pi-lens/lsp.json` config snippet registering veriplan as a custom LSP server

## Capabilities

### New Capabilities

- `lsp-transport`: stdio-based LSP transport using tower-lsp, with file-to-change resolution and workspace state management
- `lsp-diagnostics`: real-time convertibility checking on file save, mapping CheckItems to LSP Diagnostics
- `lsp-completions`: task ID and temporal keyword completions in spec files
- `lsp-navigation`: go-to-definition and hover for task references across files
- `lsp-symbols`: document symbol hierarchy for both tasks.md (phases → tasks) and spec.md (requirements → scenarios)
- `lsp-code-actions`: quick fixes surfaced from CheckItem.fix suggestions

### Modified Capabilities

- *(none)*

## Impact

- **Cargo.toml**: add `tower-lsp`, `lsp-types`, `tokio` dependencies
- **New module**: `src/lsp/` with submodules for transport, state, diagnostics, completions, navigation, symbols, code actions
- **main.rs**: new `Lsp` variant in `Commands` enum dispatching to `run_lsp()`
- **No breaking changes** — existing `check`, `init`, `visualize` subcommands unchanged
- **pi-lens integration**: optional `.pi-lens/lsp.json` config — no code change in pi-lens needed

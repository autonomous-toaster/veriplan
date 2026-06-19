## Context

Veriplan is a single Rust binary with CLI subcommands (`check`, `init`, `visualize`). The library (`src/lib.rs`) exposes the parser, checker, translator, and visualizer as public modules — the CLI is pure orchestration in `main.rs`. Adding an LSP subcommand follows the same pattern: the LSP server is a new `src/lsp/` module that reuses existing library functions and is invoked via `veriplan lsp --stdio`.

The LSP server runs as a daemon managed by the editor or pi-lens. It needs to hold state (cached PlanIRs per change directory) and react to file events (open, change, save).

## Goals / Non-Goals

**Goals:**
- Real-time diagnostics on file save for all 7 convertibility checks
- Task ID completions in spec files when typing `T` or `t`
- Temporal keyword completions after `SHALL`/`MUST`/`SHOULD`
- Go-to-definition from `T<N>` in spec files to `tasks.md`
- Hover tooltip showing task description + phase for task references
- Document symbol hierarchy (phases → tasks, requirements → scenarios)
- Code actions for fixable diagnostics (bad task refs, non-formalizable constraints)
- pi-lens `.pi-lens/lsp.json` configuration
- Graceful handling of files outside any OpenSpec change (empty diagnostics)

**Non-Goals:**
- SPIN model checking in LSP (too expensive for real-time; `workspace/executeCommand` only)
- Incremental parsing (full reparse on save is fast enough for typical change sizes)
- Multi-workspace support (one LSP server instance per project root)
- TCP transport (stdio covers all LSP clients and pi-lens)
- Results cache or persistent state (server is in-memory only; exits on client disconnect)

## Decisions

### Decision 1: tower-lsp over lsp-server

tower-lsp is the de facto standard Rust LSP library, well-maintained, with comprehensive lsp-types integration. It provides the `LanguageServer` trait where each method maps directly to an LSP protocol operation. lsp-server (used by rust-analyzer) is lower-level and requires more manual protocol handling. The async overhead of tokio+tower-lsp is negligible for our workload (sub-ms convertibility checks).

### Decision 2: Interior mutability via Arc<RwLock<ChangeStore>>

The `LanguageServer` trait methods take `&self`, not `&mut self`. All state lives behind `Arc<RwLock<ChangeStore>>`. `did_save` acquires a write lock to reparse and recheck; `completion`/`goto_definition`/`hover` acquire a read lock. Contention is minimal since diagnostics fire on save (seconds apart) and navigation queries are read-only.

### Decision 3: Full reparse on save, not incremental

Tree-sitter supports incremental parsing, but the convertibility check operates at the change-directory level (tasks.md + all spec files). Reparsing the entire change on save is ~5-10ms for typical plans (<100 tasks). The added complexity of incremental PlanIR patching isn't justified. The ChangeStore caches the full PlanIR and replaces it atomically on each save.

### Decision 4: File-to-change resolution by path walk

Given a file path like `.../openspec/changes/my-change/specs/foo/spec.md`, walk up parent directories looking for the `openspec/changes/<name>/` pattern. Extract `<name>` as the change name. This avoids any configuration file or manifest — it works purely from filesystem structure. If no match is found, the file is outside any OpenSpec change and returns empty results.

### Decision 5: No separate lsp crate, keep in src/lsp/

The LSP server isn't useful without the veriplan library's parser/checker. Keeping it as `src/lsp/` within the same binary is simpler, shares types directly, and avoids an extra maintainable interface boundary. The `main.rs` dispatch is a single `Commands::Lsp` variant.

### Decision 6: lsp_types crate for protocol types, not raw JSON-RPC

lsp_types provides strongly-typed Rust structs for every LSP request/response/document type. tower-lsp handles serialization. This catches protocol errors at compile time vs. runtime.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| tokio runtime adds binary size (~500KB) | Acceptable for a dev tool; the SPIN/gcc dependency dwarfs it |
| LSP client sends didChange with full content on every keystroke | Debounce: only run diagnostics on didSave, not didChange. Parsing on every keystroke would be wasteful |
| File-to-change resolution fails for unconventional project layouts | Fallback: scan all active changes and match by file prefix. If still no match, return empty |
| tower-lsp version drift with lsp-types | Pin compatible versions: tower-lsp 0.9 + lsp-types 0.97 |
| Multiple clients connect simultaneously | Not supported — single stdio transport. One client per process. |
| Auto-format modifies content after save | Not a veriplan concern — pi-lens or the editor manages formatting ordering |

## Open Questions

- Should `veriplan lsp` support `--tcp` in addition to `--stdio`? (No strong need — editors and pi-lens both use stdio.)
- Should we publish to a crate registry or keep as binary-only? (Binary-only — veriplan is a CLI tool, not a library.)

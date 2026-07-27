## Context

The codebase has 6 independent `truncate` functions, 5 of which use byte-level slicing (`&s[..max]`) that panics on multi-byte UTF-8 characters. 12 `.unwrap()` calls exist across the codebase, all of which would panic on error. No clippy lint rules forbid these patterns.

## Goals / Non-Goals

**Goals:**
- Add `[lints.clippy]` to `Cargo.toml` denying `unwrap_used`, `expect_used`, `panic`
- Replace all `.unwrap()` calls with proper error handling
- Fix all `truncate` implementations to handle multi-byte UTF-8 safely
- Consolidate `truncate` into a single shared utility function
- Add `check-lint-rules` Justfile recipe that CI can run
- Ensure `cargo clippy` passes with zero violations under the new lint rules

**Non-Goals:**
- Fixing the Kani harness to model non-ASCII strings (separate concern)
- Adding `#[deny(unsafe_code)]` or other safety lints beyond the three panic-related ones
- Changing the public API of any module

## Decisions

1. **Shared `truncate` utility in `src/util.rs`**
   - Currently 6 copies across the codebase → extract one correct implementation
   - Use `char_indices()` to find the last safe char boundary at or before `max`
   - Append `…` (single Unicode ellipsis) when truncated (consistent with LSP version)
   - All other modules import from the shared location

2. **Error handling strategy for `.unwrap()` replacements**
   - `src/lsp/handlers.rs` (9x `store.read()/write().unwrap()`): RwLock poison errors are unrecoverable in practice → replace with `expect("lock poisoned")` for clarity (the old lint `expect_used` would catch these, but we explicitly allow it with explanatory messages). Actually, since we're denying `expect_used` too, use `?` with `anyhow::Context` or explicit `match` + `panic!()` with a message.
   - Wait — we're denying `panic` too. So these need to either propagate the error or handle it gracefully.
   - For RwLock poison: the LSP handler functions return `Result`, so use `?` with `.map_err(|_| ...)`.
   - `src/lsp/transport.rs:41` (`serde_json::to_value`): replace with `?` since the function returns `Result`.
   - `src/annotator/mod.rs:32` (`.unwrap()` on Option): the plan lookup should return an error/fallback instead of panicking.
   - `src/checker/bfs.rs:276` (`task_ids.first().unwrap()`): return an error or handle empty case.

3. **`check-lint-rules` verbatim copy from shield**
   - Same bash script, same three lint rules
   - Runs as part of CI (verify existing CI pipeline)

## Risks / Trade-offs

- [Risk] Replacing `.unwrap()` with `?` changes function signatures → may cascade up the call stack.  
  → Mitigation: trace each call site and ensure `Result` propagation is feasible without major refactoring.
- [Risk] The LSP handler uses `store.read().unwrap()` in a deeply nested synchronous context. Converting to `?` may force restructuring of the handler flow.  
  → Mitigation: use `.lock().map_err(|e| ...)` instead of changing the control flow.
- [Risk] The `truncate` function in `visualizer/mod.rs` already handles UTF-8 correctly but uses `max.saturating_sub(1)` leaving room for `…`. The shared version should match this behavior.  
  → Not a risk, just a design constraint.
- [Risk] After adding lint rules, `cargo clippy` may fail in CI for unrelated future changes.  
  → This is the goal — failing CI is the enforcement mechanism.

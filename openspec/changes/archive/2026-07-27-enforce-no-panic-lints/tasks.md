## 1. Lint Configuration

- [x] 1.1 Add `[lints.clippy]` section to `Cargo.toml` denying `unwrap_used`, `expect_used`, `panic`
- [x] 1.2 Copy `check-lint-rules` recipe from shield's Justfile to veriplan's Justfile

## 2. Fix truncate Functions — Shared Utility

- [x] 2.1 Create `src/util.rs` with a UTF-8-safe `truncate` function using `char_indices()`
- [x] 2.2 Replace `truncate` in `src/checker/checks.rs` with import from shared utility; remove local definition
- [x] 2.3 Replace `truncate` in `src/checker/bfs.rs` with import from shared utility; remove local definition
- [x] 2.4 Replace `truncate` in `src/grounding/mod.rs` with import from shared utility; remove local definition
- [x] 2.5 Replace `truncate` in `src/lsp/completions.rs` with import from shared utility; remove local definition
- [x] 2.6 Replace `truncate` in `src/main.rs` with import from shared utility; remove local definition
- [x] 2.7 Replace `truncate` in `src/visualizer/mod.rs` with import from shared utility; remove local definition
- [x] 2.8 Verify all call sites produce expected output with ASCII and multi-byte strings

## 3. Replace unwrap() Calls

- [x] 3.1 Replace `.unwrap()` in `src/annotator/mod.rs:32` with proper error handling (return error or provide fallback)
- [x] 3.2 Replace `.unwrap()` in `src/checker/bfs.rs:276` with proper error handling (handle empty task_ids)
- [x] 3.3 Replace `.unwrap()` on `store.read()` in `src/lsp/handlers.rs` with `?` or `map_err` (9 occurrences)
- [x] 3.4 Replace `.unwrap()` on `store.write()` in `src/lsp/handlers.rs` with `?` or `map_err` (4 occurrences)
- [x] 3.5 Replace `.unwrap()` in `src/lsp/transport.rs:41` with `?`

## 4. Verification

- [x] 4.1 Run `cargo clippy` and confirm zero violations under new lint rules
- [x] 4.2 Run `cargo test` and confirm all tests pass
- [x] 4.3 Run `just check-lint-rules` and confirm it passes

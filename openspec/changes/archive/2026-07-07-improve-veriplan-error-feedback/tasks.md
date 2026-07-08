## 1. Parser — First-paragraph extraction

- [x] 1.1 Modify `extract_shall_statement()` in `src/parser/helpers.rs` to return only the first paragraph (text before first blank line or `####` heading)
- [x] 1.2 Update existing tests in `src/parser/helpers.rs` to cover multi-paragraph bodies
- [x] 1.3 Run `cargo test` to verify no regressions in spec parsing

## 2. Grounding — Multi-keyword pre-check

- [x] 2.1 In `src/grounding/mod.rs`, after the grounder returns, check if multiple predicates matched with confidence > 0.5
- [x] 2.2 Emit `grounding_ambiguous_multi_keyword` blocker with message identifying conflicting keywords
- [x] 2.3 Add unit tests for multi-keyword detection

## 3. Pipeline — Skip BFS for grounding failures

- [x] 3.1 In `src/checker/convertibility.rs`, propagate grounding failure status to the verification result
- [x] 3.2 In `src/checker/mod.rs`, skip BFS model checker for requirements with `PatternUngrounded` status
- [x] 3.3 Add integration test: requirement with grounding failure should not produce BFS violations

## 4. Error messages — Audit `suggest_fix()`

- [x] 4.1 In `src/checker/bfs.rs`, update `suggest_fix()` for `Exclusive` category to reference actual detected keywords
- [x] 4.2 Update `suggest_fix()` for `Conditional` category to reference actual detected keywords
- [x] 4.3 Update `suggest_fix()` for `SequentialOrder` category to reference actual detected keywords
- [x] 4.4 Update tests in `src/checker/bfs_tests.rs` for new message formats

## 5. CLI — Add `--change` alias

- [x] 5.1 In `src/cli.rs`, add `--change` as a visible alias for the positional `[CHANGE]` argument in the `check` subcommand
- [x] 5.2 Verify `veriplan check --change <name>` works identically to `veriplan check <name>`

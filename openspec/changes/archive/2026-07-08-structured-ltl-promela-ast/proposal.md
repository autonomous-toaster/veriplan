## Why

The LTL translator and Promela generator use strings as their intermediate representation: `generate_ltl()` returns a `String`, `generate_promela()` returns a `String`, and the BFS evaluator parses LTL back from strings using `strip_prefix()` and `split_once()`. This string-as-API pattern caused the BFS evaluator to be silently broken for months (it looked for `G ( ... )` while the translator emitted `[] ( ... )`), and it prevents Kani from verifying the translation logic because string operations create exponential state spaces in symbolic execution.

## What Changes

- Introduce `LtlFormula` and `LtlCondition` enums to replace `String` as the LTL representation
- Introduce `PromelaAst` types to replace `String` as the Promela representation
- Refactor `generate_ltl()` to return `LtlFormula` instead of `Option<String>`
- Refactor `evaluate_ltl()` to take `&LtlFormula` instead of `&str`
- Refactor `generate_promela()` to return `PromelaAst` instead of `String`
- Add thin serialization functions: `ltl_to_string()`, `promela_to_string()`
- Remove string-parsing logic from the BFS evaluator
- Update Kani harnesses to verify the new structured types

## Capabilities

### New Capabilities

- `ltl-ast`: Structured LTL formula types (`LtlFormula`, `LtlCondition`) with Kani-verifiable evaluation and serialization
- `promela-ast`: Structured Promela model types (`PromelaModel`, `PromelaStmt`) with Kani-verifiable generation and serialization

### Modified Capabilities

- `rule-translator`: `generate_ltl()` return type changes from `Option<String>` to `Option<LtlFormula>` — **BREAKING** for any code that pattern-matches on the returned string
- `model-check`: `evaluate_ltl()` signature changes from `&str` to `&LtlFormula` — **BREAKING** for the BFS checker call site

## Impact

- **New types**: `src/ir/ltl.rs` (~50 lines), `src/ir/promela.rs` (~80 lines)
- **Modified files**: `src/translator/mod.rs`, `src/checker/bfs.rs`, `src/checker/promela.rs`
- **Removed code**: String-parsing logic in `evaluate_ltl()` (~15 lines of strip_prefix/split_once)
- **Kani harnesses**: Can now verify LTL generation and evaluation by structural induction on enums
- **No external API change**: CLI output, SPIN input, and JSON output remain identical

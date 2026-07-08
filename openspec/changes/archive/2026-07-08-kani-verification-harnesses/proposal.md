## Why

Veriplan is a verification tool, but its own core translation logic (natural language → LTL → Promela) has no formal correctness guarantees. The BFS fallback checker is silently unsound — it passes all LTL properties because it looks for `G ( ... )` while the translator emits `[] ( ... )`. The naming convention between LTL variable references and Promela declarations is a fragile coupling with no regression protection. Unit tests cover individual functions but cannot prove the absence of edge-case bugs across the combinatorial space of plan structures and requirement phrasings.

Kani is a bit-precise model checker for Rust, developed at AWS, proven at scale (16K+ harnesses in Rust std lib CI). It checks safety properties with zero annotations and extends to functional correctness via contracts. Adding Kani verification harnesses to veriplan's own source code creates a trust chain: Kani (proven) → veriplan (verified) → user plans (verified).

## What Changes

- Add Kani as a dev-dependency and configure it for CI
- Write proof harnesses for the BFS LTL evaluator to prove it correctly evaluates all LTL patterns the translator generates
- Write proof harnesses for the naming convention coupling between translator and Promela generator
- Write proof harnesses for the translator's LTL generation (syntactic validity, reference consistency)
- Write proof harnesses for the Promela generator (balanced braces, variable declarations match LTL refs)
- Write proof harnesses for the convertibility check (severity consistency, no panics)
- Fix the BFS evaluator to handle `[]` and `<>` LTL patterns (proven by Kani after fix)
- Add CI step to run Kani harnesses alongside existing tests

## Capabilities

### New Capabilities

- `kani-harnesses`: Kani proof harnesses that verify veriplan's core translation and verification logic. Covers the BFS LTL evaluator, naming convention coupling, LTL generation, Promela generation, and convertibility checks.

### Modified Capabilities

- *(none — no existing spec-level behavior changes)*

## Impact

- **New dev-dependency**: `kani` crate added to `[dev-dependencies]`
- **New directory**: `kani-harnesses/` with 5 harness files (~40KB total)
- **CI change**: `cargo kani` step added to CI pipeline (estimated 2-5 min runtime)
- **Bug fix**: BFS `evaluate_ltl` function fixed to handle `[]` and `<>` patterns
- **No runtime impact**: Kani is dev-only, not shipped to users

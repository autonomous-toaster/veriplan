## Context

Veriplan's verification pipeline has three stages: PlanIR → LTL translation → Promela generation → model checking (SPIN or BFS fallback). The first two stages are pure Rust string logic with no formal correctness guarantees. The BFS fallback's `evaluate_ltl` function only handles `G ( ... )` patterns but the translator emits `[] ( ... )` — making the BFS checker silently unsound for all LTL properties. The naming convention between LTL variable references (`active_t1_3`) and Promela declarations (`bit active_t1_3`) is a fragile coupling maintained across three separate functions with no regression protection.

Kani is a bit-precise model checker for Rust that operates at the MIR level. It checks safety properties (overflow, panic, UB) with zero annotations and extends to functional correctness via function contracts, loop contracts, and quantifiers. It's proven at scale: 16K+ harnesses in Rust std lib CI, 11 bugs found in production projects.

## Goals / Non-Goals

**Goals:**
- Prove the BFS LTL evaluator correctly evaluates all LTL patterns the translator generates
- Prove the naming convention between translator and Promela generator is consistent
- Prove generated LTL formulas are syntactically valid and reference only existing tasks
- Prove generated Promela has balanced structure and declared variables match LTL refs
- Prove convertibility check severity invariants hold
- Fix the BFS evaluator to handle `[]` and `<>` patterns
- Add CI step to run Kani harnesses on every change

**Non-Goals:**
- Full functional verification of every veriplan function (scope-limited to translation pipeline)
- Verification of the SPIN binary or spin-rs library (external tools)
- Verification of the tree-sitter parser (external C library)
- Performance optimization of Kani harnesses (CI runtime <5 min is sufficient)

## Decisions

**Decision 1: Kani as dev-dependency, not build dependency**
Kani is only needed during development/CI, not at runtime. Adding it to `[dev-dependencies]` keeps the build clean and avoids bloating the shipped binary.

**Decision 2: Separate harness files under `kani-harnesses/` rather than inline `#[cfg(kani)]` modules**
Inline modules require `#[cfg(kani)]` gates throughout the source, adding noise. Separate files keep production code clean and make harnesses discoverable. Each harness file maps to one source module: `bfs_evaluator.rs` → `checker/bfs.rs`, `translator.rs` → `translator/mod.rs`, etc.

**Decision 3: Bounded verification with `#[kani::unwind(n)]` rather than unbounded contracts**
The translator and Promela generator operate on bounded inputs (task count, statement length). Bounded verification with unwind bounds (n=5 for tasks, n=10 for loops) is sufficient to prove the key invariants without the annotation overhead of function contracts. Contracts can be added later for deeper properties.

**Decision 4: Fix BFS evaluator before writing passing harnesses**
The BFS evaluator is known-broken. Writing a harness that expects it to fail, then fixing it and watching the harness pass, is the correct workflow. The fix is: add `[]` and `<>` pattern matching alongside the existing `G` matching, and add `F` matching alongside `<>`.

**Decision 5: Run Kani in CI as a separate job, not merged into `cargo test`**
Kani has different toolchain requirements (kani compiler plugin) and runtime characteristics (2-5 min vs seconds). A separate CI job keeps feedback fast for normal tests while still catching regressions.

## Risks / Trade-offs

- **[Risk] Kani version compatibility**: Kani tracks rustc nightly. If veriplan's rustc version diverges, Kani may not compile. → **Mitigation**: Pin Kani version in Cargo.lock, update in lockstep with rustc.
- **[Risk] False positives from bounded verification**: Unwind bounds may not cover all execution paths. → **Mitigation**: Choose bounds that exceed realistic inputs (max 3 tasks, max 10 loop iterations). Document bounds and increase if needed.
- **[Risk] CI runtime increase**: Kani is slower than unit tests. → **Mitigation**: Run as separate CI job. Target <5 min for 5 harnesses. Split into parallel jobs if needed.
- **[Trade-off] Harness maintenance cost**: Harnesses must be updated when the code changes. → **Mitigation**: Harnesses test stable interfaces (LTL syntax, Promela structure) that change infrequently. The BFS evaluator fix is a one-time cost.

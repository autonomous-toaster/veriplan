## Context

The current pipeline passes strings between stages:

```
classify() → ConstraintCategory (enum — good)
generate_ltl() → String (LTL formula as text)
evaluate_ltl() parses String back with strip_prefix/split_once
generate_promela() → String (Promela source as text)
SPIN parses Promela from text
```

The Kani implementation revealed two problems with this:
1. The BFS evaluator was silently broken because it looked for `G (...)` while the translator emitted `[] (...)`
2. Kani cannot verify string-heavy code — `contains()`, `format!()`, `replace()` create exponential state spaces

The fix is to introduce structured intermediate representations (enums) between stages, keeping string serialization as a thin leaf function at the end.

## Goals / Non-Goals

**Goals:**
- Replace `String` LTL with `LtlFormula`/`LtlCondition` enums in `generate_ltl()` and `evaluate_ltl()`
- Replace `String` Promela with `PromelaAst` types in `generate_promela()`
- Add thin `ltl_to_string()` and `promela_to_string()` serialization functions
- Make the LTL pipeline Kani-verifiable by structural induction on enums
- Remove all string-parsing logic from `evaluate_ltl()`

**Non-Goals:**
- Changing the SPIN binary interface (still outputs Promela text)
- Changing the CLI output format
- Changing the PlanIR or input parsing
- Full Promela AST verification in this change (scoped to LTL first)

## Decisions

**Decision 1: LtlFormula/LtlCondition as enums, not a trait**
Enums are closed and exhaustive — the compiler ensures all variants are handled. A trait with implementations would allow external extensions but make verification harder (Kani can't enumerate all possible implementations).

```rust
enum LtlFormula {
    Always(LtlCondition),
    Eventually(LtlCondition),
}

enum LtlCondition {
    Atom(String),
    Not(Box<LtlCondition>),
    And(Vec<LtlCondition>),
    Or(Vec<LtlCondition>),
    Implies(Box<LtlCondition>, Box<LtlCondition>),
    Iff(Box<LtlCondition>, Box<LtlCondition>),
}
```

**Decision 2: PromelaAst as a set of structs, not a full grammar**
The Promela subset veriplan generates is small: variable declarations, proctype definitions with do/od loops, and LTL properties. A full Promela grammar would be overkill. Simple structs with Vec<Stmt> are sufficient.

```rust
struct PromelaModel {
    variables: Vec<PromelaVar>,
    processes: Vec<PromelaProcess>,
    properties: Vec<PromelaLtl>,
}
```

**Decision 3: Serialization as a separate module, not methods on types**
`ltl_to_string()` and `promela_to_string()` are pure functions in a `serialize` module. This keeps the AST types free of formatting concerns and makes the serialization easy to test independently.

**Decision 4: LTL AST first, Promela AST second (two-phase implementation)**
The LTL pipeline is smaller, self-contained, and has the known bug (BFS evaluator). Refactoring it first provides immediate value and a template for the Promela refactoring.

## Risks / Trade-offs

- **[Risk] Migration of existing code**: All callers of `generate_ltl()` and `evaluate_ltl()` need updated signatures. → **Mitigation**: The functions have few callers (translator, BFS checker, tests). Update all in one commit.
- **[Risk] Serialization bugs**: The new `ltl_to_string()` might produce different output than the old `format!()`. → **Mitigation**: Add round-trip tests: generate LTL, serialize to string, parse back, compare ASTs.
- **[Trade-off] More types, more code**: Adding AST types increases the codebase size. → **Benefit**: Type safety catches bugs at compile time that previously required runtime testing.

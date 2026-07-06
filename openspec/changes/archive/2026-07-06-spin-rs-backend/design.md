## Context

veriplan's model checking pipeline currently has two paths:

1. **External spin binary** (`checker/spin.rs`): generates Promela → writes to temp file → `spin -a` (generates C verifier) → `gcc` (compiles pan.c) → `./pan` (runs search) → parse stdout for results
2. **BFS fallback** (`checker/bfs.rs`): naive 2^N state enumeration, only handles `G ( condition )` LTL patterns

The spin binary path is slow (compile step), fragile (binary must be on PATH), and produces opaque output that must be parsed from stdout. The BFS fallback is too limited for real use.

spin-rs is a Rust-native Promela model checker available as a library. It can parse Promela, compile to Lua, and run DFS/BFS verification with LTL→Büchi support — all in-process.

## Goals / Non-Goals

**Goals:**
- Add spin-rs as an alternative in-process model checker backend
- Allow switching between spin and spin-rs via CLI flag and env var
- Keep existing spin path completely unchanged (default behavior preserved)
- Add comparison mode to run both backends and diff results
- Share Promela generation between both backends

**Non-Goals:**
- Replacing the spin binary path entirely (it stays as default)
- Implementing a direct PlanIR→Model trait (spin-rs takes Promela input)
- Modifying the BFS fallback (it remains as-is)
- Publishing spin-rs as a crate (it stays a git dependency)

## Decisions

### Decision 1: Shared Promela generation via extracted module

`generate_promela()` currently lives in `checker/spin.rs`. Both backends need it.

**Chosen**: Move it to `checker/promela.rs` as a public function. Both `spin.rs` and `spin_rs.rs` import it.

**Alternative considered**: Keep it in `spin.rs` and call `spin::generate_promela()` from the new module. Rejected because it creates a circular-ish dependency where the new module depends on the old one, and the extracted module is cleaner.

### Decision 2: Backend selection via enum threaded through the call chain

**Chosen**: Add a `CheckerBackend` enum (`Spin` | `SpinRs`) to `checker/mod.rs`. Thread it through `verify()` and `verify_with_strictness()`. The CLI parses `--checker` into this enum. Default is `Spin`.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckerBackend {
    Spin,
    SpinRs,
}
```

### Decision 3: Comparison mode as a separate output

**Chosen**: When `--compare` is set, run both backends sequentially, then produce a side-by-side diff table showing per-constraint pass/fail status and any mismatches.

**Alternative considered**: Run both in parallel. Rejected — complexity isn't worth it for a diagnostic mode.

### Decision 4: spin-rs as git dependency

**Chosen**: Add to `Cargo.toml` as:
```toml
spin-rs = { git = "https://github.com/autonomous-toaster/spin-rs" }
```

No feature flags needed — spin-rs's default features (`lua-runtime`) are sufficient.

### Decision 5: Result mapping from CheckResult to VerificationResult

spin-rs's `CheckResult` has: `states_explored`, `states_stored`, `transitions`, `depth_reached`, `errors`, `violations`, `elapsed_secs`.

veriplan's `VerificationResult` has: `plan_name`, `phase`, `convertible`, `convertibility_report`, `valid`, `violations`, `total_constraints`, `satisfied_constraints`, `skip_reason`, `constraints_summary`.

**Mapping**:
- `CheckResult.errors == 0` → `valid = Some(true)`, else `valid = Some(false)`
- `CheckResult.violations` → map each to veriplan's `Violation` struct (extract `property_name` → `constraint_id`, `description` → `state`)
- `total_constraints` = number of LTL formulas in the Promela model
- `satisfied_constraints` = `total_constraints - violations.len()`
- `constraints_summary` = per-formula pass/fail derived from violations list

## Risks / Trade-offs

- **spin-rs correctness gap** → spin-rs may not match spin's results for all LTL patterns. Mitigation: comparison mode lets us find and fix discrepancies before switching defaults.
- **Lua runtime dependency** → spin-rs depends on `mlua` with vendored Lua 5.4, adding ~5MB to build. Acceptable for a verification tool.
- **Git dependency fragility** → unpublished crate could break if repo changes. Mitigation: pin to a specific commit in `Cargo.toml`.
- **Performance unknown for large models** → spin-rs may be slower than compiled C for very large state spaces. Mitigation: comparison mode measures this.

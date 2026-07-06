## 1. Dependency & Module Setup

- [x] 1.1 Add `spin-rs` git dependency to `Cargo.toml` pinned to a specific commit
- [x] 1.2 Create `checker/promela.rs` — extract `generate_promela()` from `spin.rs` into shared module, make it public
- [x] 1.3 Update `checker/spin.rs` to import `generate_promela()` from `promela.rs` instead of defining it locally
- [x] 1.4 Add `pub mod promela;` to `checker/mod.rs`

## 2. CheckerBackend Enum & Routing

- [x] 2.1 Add `CheckerBackend` enum (`Spin`, `SpinRs`) to `checker/mod.rs`
- [x] 2.2 Add `checker_backend` parameter to `verify()` and `verify_with_strictness()`
- [x] 2.3 Route to `spin::run_spin_check()` or new `spin_rs::run_spin_rs_check()` based on backend
- [x] 2.4 Keep `require_spin()` check only for `CheckerBackend::Spin`

## 3. spin-rs Checker Module

- [x] 3.1 Create `checker/spin_rs.rs` with `run_spin_rs_check()` function
- [x] 3.2 Implement Promela generation via `promela::generate_promela()`
- [x] 3.3 Call `spin_rs::verify(promela_str)` to run the model checker
- [x] 3.4 Map `CheckResult` to `VerificationResult` (errors → valid, violations → Violation structs, constraints_summary)
- [x] 3.5 Add `pub mod spin_rs;` to `checker/mod.rs`

## 4. CLI Flag & Env Var

- [x] 4.1 Add `--checker` CLI flag to `check` subcommand in `main.rs` (values: `spin`, `spin-rs`)
- [x] 4.2 Add `--compare` CLI flag to `check` subcommand in `main.rs`
- [x] 4.3 Parse `VERIPLAN_CHECKER` env var in `run_check()`, with CLI flag overriding env var
- [x] 4.4 Thread `CheckerBackend` through `run_check()` → `verify_with_strictness()` → `verify()`
- [x] 4.5 Validate `--checker` value and error on invalid input

## 5. Comparison Mode

- [x] 5.1 In `run_check()`, when `--compare` is set, run both backends sequentially
- [x] 5.2 Build per-constraint comparison table: `Constraint | spin | spin-rs | Match?`
- [x] 5.3 Print summary: `X/Y constraints match, Z mismatches` + elapsed times
- [x] 5.4 Exit 0 on agreement, exit with warning on mismatch (non-blocking in pre-commit)

## 6. Verification

- [x] 6.1 `cargo check` — verify compilation with spin-rs dependency
- [x] 6.2 `cargo test` — verify existing tests still pass (no regressions)
- [x] 6.3 Manual test: `veriplan check --checker spin-rs` on a known plan
- [x] 6.4 Manual test: `veriplan check --compare` on a known plan
- [x] 6.5 Manual test: `VERIPLAN_CHECKER=spin-rs veriplan check` (env var path)

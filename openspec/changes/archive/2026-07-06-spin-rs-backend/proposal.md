## Why

veriplan currently depends on an external `spin` binary for model checking — it generates Promela, writes it to a temp file, calls `spin -a` to generate C verifier source, compiles with `gcc`, then runs `./pan`. This is slow (compile step), fragile (binary must be on PATH), and limits portability. The in-process BFS fallback is too limited to be useful.

spin-rs (https://github.com/autonomous-toaster/spin-rs) is a Rust-native Promela model checker that can be used as a library. Integrating it gives us an in-process checker with no external dependencies, faster startup, and the ability to compare results against the existing spin path for validation.

## What Changes

- Add `spin-rs` as a git dependency in `Cargo.toml`
- Extract `generate_promela()` from `checker/spin.rs` into a shared module so both backends can use it
- Add new `checker/spin_rs.rs` module that calls `spin_rs::verify()` and maps results to veriplan's `VerificationResult`
- Add `--checker` CLI flag (`spin` | `spin-rs`) and `VERIPLAN_CHECKER` env var to select backend (default: `spin`)
- Add `--compare` CLI flag to run both backends and diff results
- Route backend selection through `checker/mod.rs` into `verify()` and `verify_with_strictness()`
- No changes to the existing `checker/spin.rs` module — it remains untouched

## Capabilities

### New Capabilities
- `checker-backend-selection`: CLI flag and env var to choose between spin and spin-rs backends
- `spin-rs-integration`: Promela → spin-rs library verification pipeline with result mapping
- `checker-comparison`: Run both backends on the same plan and produce a diff of results

### Modified Capabilities
<!-- No existing spec-level requirements change — this is purely an implementation change -->

## Impact

- **Dependencies**: Adds `spin-rs` git dependency (unpublished crate, repo at github.com/autonomous-toaster/spin-rs)
- **Code**: New `checker/spin_rs.rs` module, minor changes to `checker/mod.rs` for routing, `cli.rs` for new flags, `main.rs` for flag propagation
- **CLI**: New `--checker` and `--compare` flags on the `check` subcommand
- **Backward compatibility**: Default behavior unchanged (spin binary remains default)

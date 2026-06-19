## 1. Pre-commit mode flag and exit code semantics

- [x] 1.1 Add `--pre-commit` flag to `Check` variant in main.rs CLI args
- [x] 1.2 Implement pre-commit exit code mapping: blockers → 1, SPIN-missing → 0, violations → 1, valid → 0
- [x] 1.3 Modify `require_spin()` to accept a `pre_commit: bool` parameter; when true, warn instead of hard-fail
- [x] 1.4 Detect `$PRE_COMMIT` env var for concise output format (one line per change, blocker details only, skip hint)
- [x] 1.5 Wire `--pre-commit` flag through `run_check()` to the verification flow

## 2. Pre-commit hooks configuration and documentation

- [x] 2.1 Create `.pre-commit-hooks.yaml` with `veriplan` (language: rust) and `veriplan-system` (language: system) hook IDs
- [x] 2.2 Add README section on pre-commit integration (both hook IDs, SPIN setup, escape hatches)

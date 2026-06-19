# veriplan-pre-commit-hook

## Problem

Teams using veriplan have no automated way to catch spec violations before they reach code review. Developers must remember to run `veriplan check` manually, and violations are often discovered late — during CI or not at all.

The existing `veriplan check` command works well for manual use, but has a critical limitation in pre-commit contexts: it exits code 2 when SPIN is missing, which would block every commit for anyone without SPIN installed. This makes it unsuitable as a pre-commit hook without modification.

## Proposal

Add a `--pre-commit` flag to `veriplan check` that provides "sane mode" semantics for hook usage:

- **Missing SPIN → exit 0 with warning** (not exit 2) — the plan may still be valid, we just can't prove it
- **Convertibility blockers → exit 1** — blocks the commit
- **SPIN violations → exit 1** — blocks the commit
- **Warnings only → exit 0** — doesn't block the commit

Additionally, provide a `.pre-commit-hooks.yaml` in the repo root so teams can add veriplan to their pre-commit configuration with a single config entry. Two hook IDs are offered:

- `veriplan` (language: rust) — compiles from source, works everywhere
- `veriplan-system` (language: system) — assumes veriplan is already installed, faster for teams with it in PATH

The hook runs the full check (convertibility + SPIN model checking when available) on all active changes — not just staged files — because the plan is consistent as a whole and partial checks would miss cross-constraint violations.

## Scope

- New `--pre-commit` flag on the `check` subcommand
- `.pre-commit-hooks.yaml` with two hook IDs
- README documentation for pre-commit setup
- No CI/GitHub Action (out of scope for now)

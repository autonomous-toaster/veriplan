Task Reference

| T ID | Description |
|------|-------------|
| T2.1 | Create .pre-commit-hooks.yaml with veriplan and veriplan-system hook IDs |
| T2.2 | Document pre-commit setup in README |

## ADDED Requirements

### Requirement: Pre-commit hooks configuration file

T2.1 SHALL ALWAYS provide a `.pre-commit-hooks.yaml` file in the repository root with two hook entries:

- `id: veriplan` with `language: rust`, `entry: veriplan check --pre-commit`, `files: 'openspec/'`, `pass_filenames: false`
- `id: veriplan-system` with `language: system`, `entry: veriplan check --pre-commit`, `files: 'openspec/'`, `pass_filenames: false`

Both entries SHALL include `stages: [pre-commit, pre-push, manual]`.

#### Scenario: User adds veriplan via language: rust

- **WHEN** T2.1 is configured with `id: veriplan` in a project's `.pre-commit-config.yaml`
- **THEN** pre-commit SHALL compile veriplan from source and use it as the hook binary

#### Scenario: User adds veriplan via language: system

- **WHEN** T2.1 is configured with `id: veriplan-system` in a project's `.pre-commit-config.yaml`
- **THEN** pre-commit SHALL call the `veriplan` binary already present in PATH

### Requirement: README documentation for pre-commit integration

T2.2 SHALL ALWAYS include a section in README.md covering:

- How to add veriplan to `.pre-commit-config.yaml` (both hook IDs)
- What the hook checks (full verification when SPIN available, convertibility-only when not)
- SPIN installation instructions for full verification
- The `VERIPLAN_SKIP=1` escape hatch
- The `--no-verify` alternative (skips all hooks)

#### Scenario: Developer reads the README

- **WHEN** T2.2 is read by a developer setting up pre-commit hooks
- **THEN** T2.2 SHALL provide copy-paste configuration for both auto-install and system-install approaches

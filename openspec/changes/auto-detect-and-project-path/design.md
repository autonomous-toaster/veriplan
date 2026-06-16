## Context

veriplan currently takes a single required argument: a change name. It resolves this against the current project's `openspec/changes/<name>/` directory. This works for one-off checks but breaks down in CI scenarios (need to check all active changes) and cross-project scenarios (need to check another repo's plan).

The CLI uses `clap` with a `Commands::Check { change: String }` struct. The change argument flows through `run_check` → `find_change_dir` → `parser::parse_plan`. Parsing assumes the change is relative to `openspec/changes/<name>/`.

## Goals / Non-Goals

**Goals:**
- `veriplan check` with no arguments discovers and verifies all active changes
- `veriplan check path/to/project` auto-detects openspec in that directory
- `--format openspec` flag exists as a placeholder for future format backends
- Change-name mode continues to work unchanged

**Non-Goals:**
- Implementing any non-openspec format
- Verifying changes from the global spec store (specs that live outside changes)
- Auto-detecting changes in nested or nonstandard directory layouts

## Decisions

**1. Disambiguation: change name vs directory path**

The current `change` argument is ambiguous — is "my-project" a change name or a directory path? Decision: try as a change name first (current behavior). If the change directory doesn't exist AND the argument looks like a path (contains `/` or is an existing directory), treat it as a project path. If neither matches, error with clear message.

**2. No-arg mode: `veriplan check`**

When no argument is given, look for `openspec/changes/` in CWD. List all subdirectories not in `archive/`. Run `verify` on each in sequence. Collect all results and print a combined report. If one change fails convertibility, continue checking others (don't stop early).

**3. Format extensibility pattern**

Add a `--format <name>` argument where the only valid value is `"openspec"`. Internally, switch on the format to select the parser module. For now, `--format openspec` is a no-op (same as default). When a new format is added later, only a new parser module and a match arm are needed.

**4. Change iteration in checker**

The `verify` function takes a single `PlanIR`. For multi-change mode, call `verify` in a loop. Each change has its own plan, convertibility report, and results. Combine into a single `VerificationResult` with multiple change entries in JSON output.

## Risks / Trade-offs

- **[Performance] Large projects with many changes** will run model checking N times (once per change). Mitigation: changes are typically small (1-5), and each model check is fast (~13s). N changes = N × 13s.
- **[CLI ergonomics] Argument disambiguation** could confuse users who have a directory that happens to match a change name. Mitigation: try change name first, which is the common case. Directory lookup only triggers when the change name doesn't exist AND the path is plausible.
- **[Scope creep] Format extensibility** could lead to premature abstraction if we over-design the trait/interface. Mitigation: keep it minimal — a single match on a string, one arm per format. No trait, no registry, no plugin system.

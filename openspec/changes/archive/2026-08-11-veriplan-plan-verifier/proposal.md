## Why

`veriplan check` generates a detailed text report (constraints, violations, LTL formulas), but there's no visual
representation of the plan-as-state-machine. A single diagram showing phases, tasks, constraints, and model-check
results would make verification output immediately graspable — especially for spotting which constraints failed
and where in the phase structure the violation occurs.

## What Changes

- **New `veriplan visualize` subcommand**: outputs a state-machine diagram of the plan
- **One unified diagram** — no mode flags. Shows both phase structure (task ordering) and constraint edges (spec rules) in a single graph
- **Verification overlay**: if `.veriplan/results.json` exists (from a previous `check` run), constraint edges are colored green (passed) or red (violated)
- Three output formats: `mermaid` (default), `dot`, `markdown` (table). Inferred from `-o` file extension, or explicit `--format` flag
- Results cache: `check` writes `.veriplan/results.json` in the project root. `visualize` reads it silently — no results = no colors

## Capabilities

### New
- `visualize`: Combined structural + constraint graph with optional pass/fail overlay

### Modified
- `check`: Additionally writes `.veriplan/results.json` after SPIN model check completes

## Impact

- All rendering is pure string generation — no new dependencies except `--format dot` which is just `digraph { ... }` to stdout
- The diagram captures the **entire veriplan pipeline** in one visual: here's the structure (phases → tasks → order), here's the rules (constraint edges), here's the results (green/red)
- No modes, no flags beyond format/output. Simple.

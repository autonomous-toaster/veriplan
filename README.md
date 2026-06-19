# veriplan — Formal Verification for OpenSpec Plans

**veriplan** checks whether an OpenSpec plan can be built into a working
state machine, then runs model checking to prove that all requirements
hold — or tells you exactly what needs fixing.

The approach is based on the paper
[*"Specification-Driven Requirements Engineering and
Plan Verification"* (arXiv:2502.17898)](https://arxiv.org/abs/2502.17898),
which describes the pipeline this tool implements.

---

## How it works, step by step

### 1. Parse the plan (markdown → data)

veriplan reads your OpenSpec change directory: `tasks.md`, `specs/`,
`design.md`, `proposal.md`. It uses tree-sitter (a syntax-aware parser)
to turn the markdown into structured data — tasks with N.M IDs,
requirements with RFC 2119 keywords (MUST / SHALL / SHOULD / MAY),
scenarios with GIVEN / WHEN / THEN steps, and phase groupings.

If the markdown is malformed or missing required fields, parsing fails
with a clear error.

### 0. Setting up a project: `veriplan init`

Before writing any tasks or requirements, run `veriplan init` once to
embed the structural rules directly into your OpenSpec configuration:

```bash
veriplan init
```

This adds a `context` field and `rules` to `openspec/config.yaml` that
describe the temporal keyword grammar, task ID format, scenario structure,
and RFC 2119 conventions — in plain language, without mentioning Promela,
SPIN, or LTL. The idea is that whoever writes the plan (including an AI
assistant) sees these rules up front and can follow them from the start.

The config is a gentle nudge, not a straitjacket. An AI assistant may
still write requirements that don't follow the temporal grammar — that's
what the convertibility check is for. But with `init`, the rules are
there in the project configuration from day one, making it more likely
that specs come out structurally sound on the first try.

### 2. Convertibility check: "Can this plan be built?"

Before running any heavy analysis, veriplan asks seven questions:

| #  | Check | What it catches |
|----|-------|-----------------|
| 1  | Every task has a unique N.M ID | Duplicate or missing IDs |
| 2  | At least one requirement uses SHALL or MUST | Purely aspirational plans can't be verified |
| 3  | Every SHALL references an existing task ID | Requirements that talk about nothing |
| 4  | Every SHALL uses a temporal keyword | SHALLs like "the system shall be fast" can't be modelled — see the golden rules below |
| 5  | Every scenario has WHEN + THEN + keyword | Missing steps leave behaviour untested |
| 6  | Constraint diversity advisory | 20 requirements all saying "ALWAYS" is a red flag |
| 7  | Every task is referenced by at least one SHALL | Unreferenced tasks have no formal purpose |

Checks 1–5 are **blockers**: if they fail, veriplan tells you exactly
what to rephrase and where. Checks 6–7 are warnings.

**The golden rule:** every SHALL must include a temporal keyword.
Think of these as the "grammar" of a formal requirement:

| Keyword | Category | Example |
|---------|----------|---------|
| BEFORE | sequential | *T2.1 SHALL complete BEFORE T3.1* |
| CONCURRENTLY | concurrent | *T4.2 SHALL run CONCURRENTLY WITH T4.3* |
| AFTER | sequential | *T5.1 SHALL complete AFTER T4.1* |
| IF…THEN | conditional (failure‑recovery) | *IF T1.1 fails THEN T2.1 SHALL run* |
| ALWAYS | global invariant | *T6.1 SHALL ALWAYS be reachable* |
| AT MOST ONE | exclusive | *AT MOST ONE of T3.1, T3.2 SHALL be active* |

Requirements without a temporal keyword are **NonFormalizable** —
they block the pipeline. The fix is always to rewrite using one of
the six patterns above.

### 3. Translate to LTL (temporal logic)

Once the plan passes convertibility, each SHALL requirement is
translated into an **LTL formula** — a precise mathematical statement
about sequences of states. For example:

> *T2.1 SHALL complete BEFORE T3.1 SHALL run*

becomes the LTL property:

```
[](active(t3_1) -> done(t2_1))
```

("It is always true that if T3.1 is active, T2.1 must already be done.")

This step maps the six temporal categories (sequential, exclusive,
conditional, concurrent, global, fixed-time) into LTL patterns that
SPIN can check.

### 4. Model check with SPIN

veriplan generates a **Promela model** — a formal state machine
description of your plan. Each task becomes a process with three
states: inactive, active, done. Phase ordering is enforced by guards.
The model is deliberately simple: it reflects the task-phase structure,
not the spec constraints. This avoids circular reasoning.

Then SPIN (the model checker) runs every LTL property against this
model:

- **Safety properties** (things that must never happen) are checked
  with a fast bitstate search.
- **Liveness properties** (things that must eventually happen) trigger
  an acceptance-cycle search — slower but necessary.

Each property gets a 5-second timeout. If SPIN can't decide within
that window, the property is marked **unchecked** (`~`).

### 5. Read the report

veriplan outputs a summary:

```
Plan: my-change — ✓ VALID
  All constraints satisfied.
  Satisfied: 22 | Violated: 0 | Unchecked: 0 | Total: 22
```

If there are violations, each one includes:

- The requirement statement and its LTL formula
- The task IDs involved and their phase
- A suggested fix (e.g. "remove CONCURRENTLY keyword or restructure")
- For conditional constraints: which task is the trigger and which is
  the consequent

Violations mean the spec demands something the plan structure cannot
guarantee — they are spec-plan mismatches, not implementation bugs.

### 6. Visualize the plan: `veriplan visualize`

Generate a state-machine diagram of the plan from tasks.md + specs:

```
$ veriplan visualize my-change
```

Three output formats:

| Format | Output | Best for |
|--------|--------|----------|
| `mermaid` (default) | `flowchart TB` with phase subgraphs | Rendering in Obsidian, GitHub, or docs |
| `dot` | Graphviz `digraph` with clusters | Advanced graph layout with Graphviz tools |
| `markdown` | Table with task relationships and source links | Plain-text review, copy-paste into plans |

The diagram shows:

- **Phase subgraphs** — numbered groups with phase mode (`[concurrent]` if marked)
- **Task nodes** — ✅ prefix for checked/completed tasks, plain for pending
- **Structural edges** — unlabeled arrows showing phase execution order
- **Constraint edges** — dashed arrows labeled with the temporal keyword
- **Results overlay** — if `.veriplan/results.json` exists from a previous `check`,
  constraint edges are colour-coded (green = passed, red = violated, orange = timed out)

Markdown format includes a **Task Index** appendix with clickable source links
(`tasks.md#L<N>`) for every task — useful for navigation and code review.

### 7. LSP server: `veriplan lsp`

veriplan includes a built-in Language Server Protocol (LSP) server for
real-time feedback in editors that support LSP (VS Code, Neovim,
Helix, etc.). The server provides:

- **Diagnostics** — convertibility errors and warnings on save
- **Completions** — task ID suggestions (type `T`) and temporal keywords
  (type `SHALL`, `MUST`, etc.)
- **Go-to-definition** — jump from `T3.2` in a spec to its definition
  in tasks.md
- **Hover** — see task description and phase on hover over task references
- **Document symbols** — outline of phases/tasks (tasks.md) and
  requirements/scenarios (spec.md)
- **Code actions** — quick fixes for convertibility diagnostics

The LSP server runs **convertibility check only** (Phase 1). Model
checking (SPIN) is too expensive for real-time feedback.

```bash
# Start the LSP server (for editor integration)
veriplan lsp --stdio
```

#### pi-lens configuration

Create a `.pi-lens/lsp.json` in your project root:

```json
{
  "servers": {
    "veriplan": {
      "command": "veriplan",
      "args": ["lsp", "--stdio"],
      "extensions": ["tasks.md", "spec.md"],
      "rootMarkers": ["openspec/config.yaml"]
    }
  }
}
```

The `extensions` field matches files by **basename** (not file extension),
so `"tasks.md"` and `"spec.md"` are matched regardless of directory.
This is intentional: using `".md"` would activate the LSP for every
markdown file in the project, but veriplan only processes files named
`tasks.md` or `spec.md`. The `rootMarkers` field tells pi-lens where
the workspace root is.

---

## Requirements

- **SPIN** (model checker) — must be on PATH. Install via
  `brew install spin` (macOS) or `apt install spin` (Debian/Ubuntu).
- **gcc** — SPIN generates C code that must be compiled.
  Both `gcc` and `spin` are checked at runtime; missing either is a
  hard failure (exit code 2).
- **Rust toolchain** (for building) — `cargo build --release`.

---

## Quick start

**Before using veriplan, install SPIN and gcc:**

```bash
# macOS
brew install spin gcc

# Debian / Ubuntu
sudo apt install spin gcc
```

SPIN is the model checker that runs the formal proofs. gcc compiles
SPIN's generated C code. Both are checked at startup — missing either
produces a clear error with install instructions.

```bash
# Build
cargo build --release

# Check a change in the current project
./target/release/veriplan check my-change-name

# Check all active changes
./target/release/veriplan check

# Run convertibility check only (no SPIN)
./target/release/veriplan check my-change --phase convertibility

# Check a change in an external project
./target/release/veriplan check /path/to/project

# JSON output for machine consumption
./target/release/veriplan check my-change --format json

# Verbose mode (see tasks, requirements, temporal classifications)
./target/release/veriplan check my-change --verbose

# Auto-configure an OpenSpec project
./target/release/veriplan init

# Generate a state-machine diagram
./target/release/veriplan visualize my-change

# Alternative formats
./target/release/veriplan visualize my-change --format dot
./target/release/veriplan visualize my-change --format markdown

# Write to a file
./target/release/veriplan visualize my-change -o plan-diagram.md

# Start LSP server for editor integration
./target/release/veriplan lsp --stdio
```
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Plan is valid — all requirements are satisfied |
| 1 | Plan is invalid — one or more requirements are violated |
| 2 | Plan is not convertible — blocking issues found |

---

## How the model works

Imagine your plan as a row of dominoes arranged in phases. Tasks
within a phase can fall one after another. Phases happen in order.

veriplan builds a simplified version of this domino row, then asks:
"If I run this row, will every requirement actually hold?"

- If the spec says "T2.1 BEFORE T3.1", veriplan checks that T3.1
  never starts before T2.1 finishes.
- If the spec says "T4.2 CONCURRENTLY WITH T4.3", veriplan checks
  that the plan structure allows them to overlap.
- If the spec says "IF T1.1 fails THEN T2.1 runs", veriplan checks
  that T2.1 actually activates when T1.1 fails — but since the model
  uses non-deterministic failure, this requires a liveness check.

The model is deliberately minimal: it only encodes the task-phase
structure. The spec constraints are checked *against* this model,
not baked into it. This catches genuine spec-plan mismatches.

If your spec is valid but the plan can't satisfy it, veriplan tells
you exactly which requirement is unrealistic and why.

---

## Project structure

```
src/
  parser/      — Parse OpenSpec markdown into structured data
  ir/          — Intermediate representation (tasks, requirements, phases)
  checker/     — Convertibility checks + SPIN orchestration
  translator/  — Map SHALL statements to LTL formulas
  visualizer/  — Generate diagrams (Mermaid, DOT, markdown)
  lsp/         — Language Server Protocol (diagnostics, completions, navigation)
  annotator/   — Human-readable and JSON report formatting
  main.rs      — CLI entry point
```

## Related

- [arXiv:2502.17898 — Specification-Driven Requirements Engineering
  and Plan Verification](https://arxiv.org/abs/2502.17898)
- [SPIN model checker](https://spinroot.com/)
- [OpenSpec](https://github.com/earendil-works/openspec)

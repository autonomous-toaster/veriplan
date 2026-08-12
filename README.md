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

### 1. Parse the plan (markdown → data)

veriplan reads your OpenSpec change directory: `tasks.md`, `specs/`,
`design.md`, `proposal.md`. It uses tree-sitter (a syntax-aware parser)
to turn the markdown into structured data — tasks with N.M IDs,
requirements with RFC 2119 keywords (MUST / SHALL / SHOULD / MAY),
scenarios with GIVEN / WHEN / THEN steps, and phase groupings.

If the markdown is malformed or missing required fields, parsing fails
with a clear error.

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

### 3. Grounding check: "Does every requirement refer to real tasks?"

Before translating to LTL, veriplan runs a **grounding** pass that
maps each requirement's SHALL statement to actual task IDs using
keyword matching and positional heuristics. This uses the
[groundcontrol](https://github.com/autonomous-toaster/groundcontrol)
library's `RuleGrounder`.

Each requirement is checked against the plan's task signature:
- Every task reference in a SHALL statement is resolved to a real task ID
- Ambiguous references (e.g. "the system" without a task) are flagged
- The grounding status (FullyGrounded, PartiallyGrounded, or
  NotGrounded) is reported per requirement

This catches requirements that talk about tasks that don't exist,
or use vague language that can't be formally checked.

### 3.5 Prose-guidance and ambiguity detection

A spec that is *formally* valid can still be *ambiguous* — and an
ambiguous spec is a liability. veriplan runs a prose-guidance pass
(backed by the [steve](https://github.com/autonomous-toaster/steve)
ASD-STE100 engine) over the human-readable parts of the plan to catch
wording that a reader (or an implementer) could interpret in more than
one way.

**Why ambiguity matters.** The whole point of a spec is to be a
single, unambiguous contract between intent and implementation. When a
requirement or scenario is vague:

- **Two implementers build different things.** "The system SHALL respond
  appropriately" means nothing concrete — one engineer ships a 200ms
  response, another ships a 2s one. The spec fails its job.
- **Verification is impossible.** If the expected behavior isn't pinned
down, there is no objective test that can say "implemented" or "not
implemented". The scenario is the executable contract; a vague scenario
is a contract that can't be enforced.
- **Change is dangerous.** A later change to an ambiguous requirement
can silently alter behavior nobody intended, because the original intent
was never captured precisely.

**What is checked.** The prose pass applies a curated set of STE rules
to four zones:

| Zone | Rules | Why |
|------|-------|-----|
| Requirement bodies (spec.md) | full curated set | the core contract |
| Task descriptions (tasks.md) | minimal (one-instruction, hedging) | grounding aliases |
| Design / proposal bodies | light (passive, pronoun, hedging) | narrative, human-oriented |
| **Scenario steps** | **PronounAmbiguity + SentenceLength** | the executable contract |

**Scenario steps are the executable contract.** A scenario's
`**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**` steps pin down the concrete
behavior an implementation must satisfy. veriplan strips the scaffolding
and code spans, then checks the remaining prose with a **safe subset** of
STE rules:

- **PronounAmbiguity** — "the valve and the pump are connected, and *it*
is faulty" is genuinely ambiguous: which one is faulty? A pronoun with
two plausible antecedents makes the expected behavior unclear.
- **SentenceLength** — an over-long step is hard to verify.

PassiveVoice and OneInstructionPerSentence are deliberately **excluded**
from scenario steps: a legitimate state assertion like "**THEN** the
plan SHALL be marked VALID" is passive by nature, and a step with two
assertions is often intentional. Flagging those would be noise.

**Advisory, never blocking.** Prose findings are surfaced as warnings
and rephrase directives — they never block the pipeline. The structural
and semantic checks (convertibility, grounding, model check) are what
gate a plan. Prose guidance is a nudge toward clarity, not a gate.

**Ambiguity detection is a two-layer defense.** The grounding check
catches *semantic* ambiguity (a requirement that can't be mapped to real
tasks). The prose pass catches *stylistic* ambiguity (wording that a
reader could interpret multiple ways). Together they make a spec both
machine-checkable and human-unambiguous.

### 4. Translate to LTL (temporal logic)

Once the plan passes convertibility and grounding, each SHALL
requirement is translated into an **LTL formula** — a precise
mathematical statement about sequences of states. For example:

> *T2.1 SHALL complete BEFORE T3.1 SHALL run*

becomes the LTL property:

```
[](active(t3_1) -> done(t2_1))
```

("It is always true that if T3.1 is active, T2.1 must already be done.")

This step maps the six temporal categories (sequential, exclusive,
conditional, concurrent, global, fixed-time) into LTL patterns that
can be checked by either the built-in BFS checker or SPIN.

### 5. Model check (BFS, SPIN, or spin-rs)

veriplan offers three model checking backends:

#### Built-in BFS checker (default)

A fast, built-in breadth-first search that explores all possible
task execution sequences up to 2^N states (N = number of tasks).
No external dependencies required. The BFS checker:

- Builds a state space where each task is inactive, active, or done
- Enforces phase ordering (sequential phases execute in order,
  concurrent phases allow overlap)
- Evaluates every LTL property against each reachable state
- Reports violations with the offending state and the property that failed

Suitable for plans with up to ~20 tasks. For larger plans, use SPIN or spin-rs.

#### SPIN model checker (optional, default)

For larger plans or when deeper liveness analysis is needed, veriplan
generates a **Promela model** and runs SPIN:

- **Safety properties** (things that must never happen) are checked
  with a fast bitstate search.
- **Liveness properties** (things that must eventually happen) trigger
  an acceptance-cycle search — slower but necessary.

Each property gets a 5-second timeout. If SPIN can't decide within
that window, the property is marked **unchecked** (`~`).

SPIN must be installed separately (`brew install spin` / `apt install spin`).
If SPIN is not available, veriplan falls back to the BFS checker automatically.

#### spin-rs model checker (in-process, optional)

veriplan also supports **spin-rs**, a Rust-native Promela model checker
that runs entirely in-process with no external dependencies. It parses
the same Promela model, compiles it to Lua, and runs DFS/BFS verification
with LTL→Büchi support via nested DFS.

To use spin-rs instead of the external SPIN binary:

```bash
# Via CLI flag
veriplan check my-change --checker spin-rs

# Via environment variable
VERIPLAN_CHECKER=spin-rs veriplan check my-change
```

spin-rs is significantly faster for small-to-medium models because it
skips the `spin -a` + `gcc` compile step. It also works on systems
where SPIN is not installed.

#### Comparing backends: `--compare`

Run both SPIN and spin-rs on the same plan and diff the results:

```bash
veriplan check my-change --compare
```

Output:
```
═══ Backend Comparison: my-change ═══

Constraint                     spin       spin-rs    Match?
------------------------------------------------------------
checker-backend-selection::…   pass       pass       ✓
...

spin:    0.55s  |  valid=✓  |  violations=0
spin-rs: 0.01s  |  valid=✓  |  violations=0

11/11 constraints match, 0 mismatches
```

Useful for validating spin-rs correctness against SPIN, or for
performance benchmarking.

### 6. Read the report

veriplan outputs a summary with three phases:

```
Plan: my-change — ✓ VALID
  Convertibility: 7/7 passed
  Grounding:      5/5 fully grounded
  Model check:    22/22 satisfied
  Satisfied: 22 | Violated: 0 | Unchecked: 0 | Total: 22
```

If there are violations, each one includes:

- The requirement statement and its LTL formula
- The task IDs involved and their phase
- A suggested fix (e.g. "remove CONCURRENTLY keyword or restructure")
- For conditional constraints: which task is the trigger and which is
  the consequent
- The grounding status (which task references were resolved)

Violations mean the spec demands something the plan structure cannot
guarantee — they are spec-plan mismatches, not implementation bugs.

### 6. Visualize the plan: `veriplan visualize`

Generate a state-machine diagram of the plan from tasks.md + specs:

```
veriplan visualize my-change
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

### 8. Pre-commit hook

veriplan can run as a **pre-commit hook** to catch spec violations
before they reach code review. The hook runs the full verification
pipeline (convertibility + SPIN model checking when available).

```bash
# Run manually with pre-commit mode
veriplan check --pre-commit

# Or set the environment variable
PRE_COMMIT=1 veriplan check
```

In pre-commit mode:

- **Blockers and violations** → exit 1 (blocks the commit)
- **Warnings** → exit 0 (doesn't block the commit)
- **SPIN not found** → exit 0 with warning (doesn't block the commit)

This is different from normal `veriplan check` where missing SPIN
causes exit code 2 (hard failure). In pre-commit mode, you don't
want to block every commit just because someone doesn't have SPIN
installed locally.

#### Using the pre-commit framework

Add to your `.pre-commit-config.yaml`:

**Option A: Auto-install (compiles from source)**

```yaml
repos:
  - repo: https://github.com/autonomous-toaster/veriplan
    rev: v0.1.0  # Use the latest tag
    hooks:
      - id: veriplan
```

**Option B: System install (veriplan must be in PATH)**

```yaml
repos:
  - repo: local
    hooks:
      - id: veriplan
        name: veriplan
        entry: veriplan check --pre-commit
        language: system
        files: 'openspec/'
        pass_filenames: false
        stages: [pre-commit, pre-push, manual]
```

**Option C: Reference the repo's hook IDs**

```yaml
repos:
  - repo: https://github.com/autonomous-toaster/veriplan
    rev: v0.1.0
    hooks:
      - id: veriplan-system  # Uses veriplan from PATH
```

The hook only runs when `openspec/` files are staged — other commits
skip it entirely. veriplan auto-detects which changes to check.

To skip the hook for a specific commit:

```bash
VERIPLAN_SKIP=1 git commit -m "work in progress"
# or skip all hooks:
git commit --no-verify -m "work in progress"
```

**For CI:** Run `veriplan check` (without `--pre-commit`) for full
verification including SPIN model checking. The pre-commit hook is a
fast guard rail; CI is the authoritative check.

---

## Requirements

- **Rust toolchain** (for building) — `cargo build --release`.
- **SPIN** (model checker, optional) — must be on PATH for SPIN-based
  model checking. Install via `brew install spin` (macOS) or
  `apt install spin` (Debian/Ubuntu). veriplan includes a built-in
  BFS checker and a spin-rs backend that work without SPIN.
- **gcc** (optional) — SPIN generates C code that must be compiled.
  Not needed when using the built-in BFS checker or spin-rs backend.

---

## Quick start

```bash
# Build
cargo build --release

# Check a change in the current project (full pipeline)
./target/release/veriplan check my-change-name

# Check all active changes
./target/release/veriplan check

# Run a specific phase only
./target/release/veriplan check my-change --phase convertibility
./target/release/veriplan check my-change --phase grounding
./target/release/veriplan check my-change --phase model-check

# Check a change in an external project
./target/release/veriplan check /path/to/project

# Use spin-rs backend (in-process, no external SPIN needed)
./target/release/veriplan check my-change --checker spin-rs

# Compare SPIN and spin-rs results
./target/release/veriplan check my-change --compare

# Set spin-rs as default via environment variable
export VERIPLAN_CHECKER=spin-rs
./target/release/veriplan check my-change

# JSON output for machine consumption
./target/release/veriplan check my-change --format json

# Verbose mode (see tasks, requirements, temporal classifications, grounding)
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
  checker/     — Convertibility checks + BFS/SPIN/spin-rs model checking
    promela.rs   — Shared Promela generation for all backends
    spin.rs      — External SPIN binary backend
    spin_rs.rs   — In-process spin-rs library backend
    bfs.rs       — Built-in BFS fallback checker
  translator/  — Map SHALL statements to LTL formulas
  grounding/   — Ground requirements against plan task signature
  visualizer/  — Generate diagrams (Mermaid, DOT, markdown)
  lsp/         — Language Server Protocol (diagnostics, completions, navigation)
  annotator/   — Human-readable and JSON report formatting
  input/       — Plan loading, source resolution, strictness profiles
  main.rs      — CLI entry point
  kani_harnesses/ — Kani proof harnesses (behind `#[cfg(kani)]`)

```

## Formal verification with Kani

veriplan uses [Kani](https://model-checking.github.io/kani/), a bit-precise
model checker for Rust, to verify its own core translation logic.

### What Kani proves

The BFS LTL evaluator (`src/checker/bfs.rs`) is verified by structural
induction on the `LtlFormula`/`LtlCondition` enums:

| Property | Harness | Time |
|---|---|---|
| Variable lookup (present) | `verify_atom_present` | 0.9s |
| Variable lookup (absent) | `verify_atom_absent` | 0.9s |
| Negation | `verify_atom_negation` | 1.1s |
| Implication (true) | `verify_implication_true` | 2.0s |
| Implication (false) | `verify_implication_false` | 2.5s |
| Always(Atom) | `verify_always_atom` | 1.9s |
| Always(Not) | `verify_always_not` | 2.7s |
| Always(Eventually) | `verify_always_eventually` | 2.8s |
| Always(Iff) | `verify_always_iff` | 3.6s |
| Always(And) | `verify_always_and` | 3.9s |

### Running harnesses

```bash
# Requires Kani 0.67+: cargo install kani-verifier
just kani
# Or run individual harnesses:
cargo kani --harness verify_always_and --unwind 10
```

### Architecture

LTL formulas use structured enums (`LtlFormula`/`LtlCondition` in
`src/ir/ltl.rs`) instead of raw strings. This makes the evaluation
verifiable by structural induction and prevents the class of bugs where
string format mismatches cause silent verification failures.

```
classify() ──▶ generate_ltl() ──▶ LtlFormula ──▶ evaluate_ltl()
                   │                  │              │
                   │           ltl_to_string()   match on variants
                   │                  │         (Kani-verifiable)
                   ▼                  ▼
              TranslatedConstraint   SPIN input
```

## Related

- [arXiv:2502.17898 — Specification-Driven Requirements Engineering
  and Plan Verification](https://arxiv.org/abs/2502.17898)
- [SPIN model checker](https://spinroot.com/)
- [OpenSpec](https://github.com/earendil-works/openspec)

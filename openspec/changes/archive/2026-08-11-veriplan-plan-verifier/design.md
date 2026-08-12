## Context

veriplan is a CLI tool that validates OpenSpec plan files for convertibility to a formal model, then performs SPIN model checking. It implements the VeriPlan paper's approach (arXiv 2502.17898) adapted for OpenSpec's actual format structure.

The core insight: OpenSpec specs describe system behavior through SHALL statements and scenarios — these map directly to the 6 VeriPlan temporal constraint categories. But the AI assistant often writes vague specs that can't be formalized. veriplan catches this, tells the AI exactly what to rephrase, and closes the loop.

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                        veriplan Pipeline                              │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  OpenSpec change dir                                                  │
│  ├── tasks.md (N.M tasks)                                             │
│  └── specs/<name>/spec.md (SHALL + RFC 2119 + Scenarios)              │
│         │                                                             │
│         ▼                                                             │
│  ┌─────────────────┐                                                  │
│  │  Plan Parser    │  tree-sitter → PlanIR (tasks + reqs + scenarios) │
│  │  (Phase 0)      │  Every element has SourceLocation                 │
│  └────────┬────────┘                                                  │
│           │                                                           │
│           ▼                                                           │
│  ┌──────────────────────────┐                                         │
│  │  Convertibility Check    │  Can I build state machine + LTL?       │
│  │  (Phase 1 — blocking)    │  ─ blockers: bad IDs, missing SHALLs    │
│  │                         │  ─ warnings: coverage gaps, weak cats    │
│  │                         │  ─ produces AI rephrase directives       │
│  └────────┬───────────────┘                                           │
│           │                                                           │
│      [blocking?] ──→ AI rephrases spec ←───────────────────────────── │
│           │ (pass)                                                     │
│           ▼                                                           │
│  ┌──────────────────┐                                                 │
│  │  Rule Translator  │  SHALL/MUST/SHOULD → 6 temporal categories     │
│  │  (Phase 2a)       │  → LTL formulas per VeriPlan Table 1          │
│  └───────┬──────────┘                                                 │
│          │                                                            │
│          ▼                                                            │
│  ┌──────────────────┐                                                 │
│  │  SM Builder       │  PlanIR → Promela state machine                 │
│  │  (Phase 2b)       │  Boolean vars for tasks, transitions, phases   │
│  └───────┬──────────┘                                                 │
│          │                                                            │
│          ▼                                                            │
│  ┌──────────────────┐                                                 │
│  │  Model Checker    │  SPIN (preferred) or built-in BFS explorer      │
│  │  (Phase 2c)       │  Checks: ordering, safety, liveness, deadlock  │
│  └───────┬──────────┘                                                 │
│          │                                                            │
│          ▼                                                            │
│  ┌──────────────────┐                                                 │
│  │  Annotator        │  Counterexample trace → source locations        │
│  │  (Phase 2d)       │  Human-readable + JSON output                   │
│  └──────────────────┘                                                 │
└──────────────────────────────────────────────────────────────────────┘
```

## Key Data Structures

```rust
struct PlanIR {
    tasks: Vec<Task>,              // from tasks.md with N.M IDs
    requirements: Vec<Requirement>, // SHALL/MUST/SHOULD paragraphs
    scenarios: Vec<Scenario>,       // GIVEN/WHEN/THEN blocks
    phases: Vec<Phase>,            // section groupings
    source_map: SourceMap,         // element ID ↔ file location
}

struct Task {
    id: String,                    // "1.3"
    description: String,
    phase: String,
    checked: bool,
    source: SourceLocation,
}

struct Requirement {
    id: String,
    statement: String,
    strength: Rfc2119Strength,     // MUST | SHOULD | MAY | MUST_NOT
    category: ConstraintCategory,  // Sequential | Conditional | ...
    ltl: Option<String>,
    sources: Vec<SourceLocation>,
}

enum Rfc2119Strength {
    Must,       // hard constraint — blocks if violated
    Should,     // soft constraint — flags if violated
    May,        // informational — not checked
    MustNot,    // hard prohibition — blocks if true
}
```

## Convertibility Check (Phase 1)

Checks run in order, earlier failures block later ones:

| # | Check | Blocking? | Failure output |
|---|-------|-----------|----------------|
| 1 | Tasks have unique N.M IDs | BLOCKING | "Task 1.3 repeated" |
| 2 | Tasks form an ordering | BLOCKING | "Task 2.4 isolated — no sequence relation" |
| 3 | At least one SHALL exists | BLOCKING | "No SHALL statements found" |
| 4 | SHALLs reference existing tasks | BLOCKING | "SHALL references T99 but max is T14" |
| 5 | SHALLs classifiable to category | BLOCKING | "'System SHALL be robust' — no temporal category" |
| 6 | Scenarios have WHEN+THEN+SHALL | WARNING | "Scenario 'Rollback' missing THEN" |
| 7 | Constraint diversity | WARNING | "All 5 constraints are Sequential — add Exclusive/Conditional" |
| 8 | RFC 2119 usage | INFO | "All SHALLs — consider MUST/SHOULD/MAY hierarchy" |

## Phase 2: Model Checking with SPIN

SPIN replaces PRISM/Storm from the VeriPlan paper. Benefits:
- Native LTL support (SPIN's `ltl` primitive in Promela)
- Explicit state space exploration with bitstate hashing
- No external dependencies (spin binary is a single file)
- Proven at scale (used for NASA verification)

Fallback: built-in BFS explorer for ≤20 tasks when SPIN unavailable.

## AI Feedback Format

```json
{
  "phase": "convertibility",
  "status": "blocking",
  "blockers": [
    {
      "check": "shall_not_classifiable",
      "element": "Requirement 'System robustness'",
      "location": "specs/plan-model/spec.md:24",
      "detail": "SHALL 'The system SHALL be robust' does not match any temporal category",
      "fix": "Rewrite using one of: sequential, exclusive, conditional, concurrent, global, fixed-time"
    }
  ],
  "warnings": [...],
  "rephrase_directive": "Rewrite requirement 3 using a sequential constraint with task references: \"T6 (Smoke tests) SHALL complete before T8 (Canary deploy)\""
}
```

## Decisions

### Decision 1: Real OpenSpec format (not fake deployment plans)

Parser reads `specs/<name>/spec.md` with `## ADDED/MODIFIED/REMOVED Requirements`, `### Requirement: Name`, `#### Scenario: Name` with GIVEN/WHEN/THEN/AND steps. Tasks use N.M numbering from `tasks.md`. This matches what `openspec validate` expects.

### Decision 2: SPIN over PRISM/Storm

SPIN is lighter-weight (single binary, no Python dependencies), has native Promela + LTL, and avoids the PRISM runtime. The VeriPlan paper used PRISM because their LLM translated to PRISM directly — we translate to Promela.

### Decision 3: RFC 2119 enforcement

OpenSpec spec documents support MUST/SHALL/SHOULD/MAY per RFC 2119. We enforce this hierarchy and map it to constraint strictness levels. Real projects almost exclusively use SHALL — we provide the incentive to use the full hierarchy.

### Decision 4: Built-in BFS explorer as fallback

Plans up to ~20 tasks (2²⁰ = 1M states) are explorable with a simple BFS in Rust. For larger plans, SPIN handles the state space. No PRISM/Storm dependency required.

## Non-Goals
- No modifications to OpenSpec CLI or format
- No timing/duration checks (AI estimates are unreliable)
- No resource allocation or scheduling
- No coverage analysis of scenarios vs requirements (future work)

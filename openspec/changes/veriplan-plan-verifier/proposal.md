## Why

OpenSpec changes define structured behavioral specs with Requirements (SHALL/MUST/SHOULD/MAY paragraphs) and Scenarios (GIVEN/WHEN/THEN steps), but there's no automated way to verify their consistency or convertibility to a formal model. Building on VeriPlan (arXiv 2502.17898), we apply formal verification — state machine modeling + LTL model checking — to OpenSpec plan files. The critical insight: the first step is a **convertibility check** that validates whether the plan is precise enough to build a formal model, then feeds back rephrase instructions to the AI assistant. Once convertible, we run full model checking (ordering consistency, constraint satisfiability, deadlock detection).

## What Changes

- **veriplan** CLI: a new Rust tool that reads real OpenSpec changes (`openspec/changes/<name>/specs/<capability>/spec.md`, `tasks.md` with N.M numbering), runs a convertibility check, produces rephrase instructions for the AI, and performs SPIN model checking on the resulting state machine
- **Convertibility check (Phase 1)**: validates task structure (unique N.M IDs, sequencing), SHALL structure (RFC 2119 keywords → 6 VeriPlan temporal categories), scenario completeness (WHEN + THEN with SHALL), and cross-spec consistency
- **SPIN integration (Phase 2)**: generates Promela state machine models from PlanIR, checks LTL properties with SPIN (replacing PRISM/Storm from the paper)
- **RFC 2119 enforcement**: maps MUST/SHALL (hard), SHOULD (soft), MUST NOT/SHALL NOT (hard prohibited), MAY (informational) to constraint strictness levels
- **AI feedback loop**: outputs structured rephrase directives telling the assistant exactly which SHALLs need rewriting, what temporal categories to use, and which OpenSpec format rules were violated

## Capabilities

### New Capabilities
- `plan-parser`: Tree-sitter markdown extraction of real OpenSpec format — `specs/<name>/spec.md` with `## ADDED/MODIFIED/REMOVED Requirements`, `### Requirement: Name`, `#### Scenario: Name` with GIVEN/WHEN/THEN/AND steps, and `tasks.md` with `N.M` task numbering
- `convertibility-check`: Validates the plan is formable into a state machine — task IDs unique, SHALLs reference existing tasks, SHALLs map to temporal categories, scenarios have complete steps, RFC 2119 keywords used consistently
- `rule-translator`: Maps SHALL/MUST/SHOULD/MUST NOT statements to 6 VeriPlan temporal categories and generates LTL formulas
- `model-check`: Promela module generation + SPIN model checking for ordering, safety, liveness, and deadlock
- `ai-feedback`: Produces structured rephrase directives targeting the AI assistant with specific OpenSpec format instructions

### Modified Capabilities
*(none — new project)*

## Impact

- New Rust project under `../veriplan/` — standalone crate
- Depends on: tree-sitter-language-pack (GFM markdown), SPIN (external model checker, optional — built-in BFS explorer for ≤20 tasks)
- Understands real OpenSpec format: reads `openspec/changes/<name>/tasks.md` and `openspec/changes/<name>/specs/<name>/spec.md`
- Does NOT modify the official OpenSpec CLI or format — sits alongside as an independent quality gate

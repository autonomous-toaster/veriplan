## 1. Project Scaffold

- [x] 1.1 Create Rust project with Cargo: `cargo init --bin` for the `veriplan` CLI
- [x] 1.2 Add dependencies to Cargo.toml: tree-sitter, tree-sitter-language-pack, serde, serde_yaml, clap
- [x] 1.3 Set up CI pipeline: cargo fmt --check, cargo clippy, cargo test
- [x] 1.4 Create module structure: `parser/`, `translator/`, `checker/`, `ir/`

## 2. PlanIR (Intermediate Representation)

- [x] 2.1 Define `Task` struct with id (N.M), description, phase, checked, source_location
- [x] 2.2 Add `Rfc2119Strength` enum: Must, Should, May, MustNot, None
- [x] 2.3 Define `Requirement` struct with id, statement, strength, category, ltl, scenarios, source_location
- [x] 2.4 Define `Scenario` struct with steps (Given/When/Then/And), source_location
- [x] 2.5 Define `SourceLocation` struct with file, start_byte, end_byte, start_line, end_line
- [x] 2.6 Define `ConvertibilityStatus` enum: Blocking, ConvertibleWithWarnings, Convertible
- [x] 2.7 Define `ConvertibilityReport` struct with blockers, warnings, rephrase_directives
- [x] 2.8 Define `SourceMap` for bidirectional lookup (element → file location)
- [x] 2.9 Define `PlanIR` struct aggregating tasks, requirements, scenarios, phases, source_map
- [x] 2.10 Define serialization for PlanIR (JSON export)

## 3. Plan Parser (real OpenSpec format)

- [x] 3.1 Initialize tree-sitter markdown grammar via tree-sitter-language-pack
- [x] 3.2 Implement `locate_change(change_root: &Path) -> Result<ChangeLayout>` that finds tasks.md and specs/**/*.md
- [x] 3.3 Implement `parse_tasks(source: &str) -> Vec<Task>` extracting N.M IDs from checklist items
- [x] 3.4 Extract section headings as phase groupings from tasks.md
- [x] 3.5 Implement `parse_requirements(source: &str) -> Vec<Requirement>` extracting `### Requirement:` sections with SHALL/MUST/SHOULD/MAY paragraphs
- [x] 3.6 Implement RFC 2119 strength detection in requirement text (MUST/SHALL/SHOULD/MAY/MUST NOT/SHALL NOT)
- [x] 3.7 Implement `parse_scenarios(source: &str) -> Vec<Scenario>` extracting `#### Scenario:` with GIVEN/WHEN/THEN/AND steps
- [x] 3.8 Implement `parse_spec` for one spec file
- [x] 3.9 Implement `parse_plan` that reads tasks.md and all specs/**/*.md
- [x] 3.10 Handle parse errors: missing files, invalid markdown, duplicate task IDs

## 4. Convertibility Check (Phase 1)

- [x] 4.1 Implement task structure checks: unique N.M IDs, at least one task exists
- [x] 4.2 Implement requirement structure checks: at least one SHALL/MUST exists, classifyable into temporal category
- [x] 4.3 Implement task reference resolution: verify SHALLs reference existing task IDs
- [x] 4.4 Implement temporal category classifier: pattern-match SHALL text against 6 VeriPlan categories
- [x] 4.5 Implement RFC 2119 coverage check: warn if all requirements use the same keyword
- [x] 4.6 Implement scenario completeness check: verify WHEN + THEN + SHALL presence
- [x] 4.7 Implement constraint diversity advisory: report distribution of temporal categories
- [x] 4.8 Implement non-formalizable SHALL flagging with human-review messages
- [x] 4.9 Implement feedback report generation with blockers, warnings, and rephrase directives
- [x] 4.10 Implement `veriplan check --phase convertibility` subcommand that runs only Phase 1

## 5. Rule Translator (LTL generation)

- [x] 5.1 Map Rfc2119Strength to strictness level: Must=hard, Should=soft, May=info, MustNot=hard inverted
- [x] 5.2 Implement sequential order SHALL → LTL: `G (active(Y) → done(X))`
- [x] 5.3 Implement exclusive constraint SHALL → LTL: `G (¬(active(X) ∧ active(Y)))`
- [x] 5.4 Implement conditional SHALL → LTL: `G (failure(X) → F active(Y))`
- [x] 5.5 Implement concurrent events SHALL → LTL: `G (active(X) ↔ active(Y))`
- [x] 5.6 Implement global invariant SHALL → LTL: `G condition`
- [x] 5.7 Implement fixed-time constraint → LTL (wall clock, no duration estimates)
- [x] 5.8 Implement flagging for unverifiable requirements (NonFormalizable)

## 6. Model Checker (Promela + SPIN)

- [x] 6.1 Implement Promela module generator: PlanIR → .pml with Boolean task variables and transitions
- [x] 6.2 Implement phase decomposition: guard conditions between phases
- [x] 6.3 Implement LTL property generator: translated LTL → Promela `ltl` declarations
- [x] 6.4 Implement SPIN subprocess runner: `spin -run -ltp -g model.pml`
- [x] 6.5 Parse SPIN stdout for property pass/fail and counterexample states
- [x] 6.6 Parse SPIN trail files for counterexample traces
- [x] 6.7 Implement built-in BFS fallback for ≤20 tasks when SPIN unavailable
- [x] 6.8 Implement deadlock detection via LTL: `G (¬deadlock)`

## 7. Annotator + Reporter

- [x] 7.1 Implement trace-to-state projection: map model checker trace → PlanIR task states
- [x] 7.2 Map each violating state to source location via SourceMap
- [x] 7.3 Generate human-readable report: per-violation source location, state, suggested fix
- [x] 7.4 Generate JSON report for CI integration
- [x] 7.5 Generate convertibility feedback as structured rephrase directives for the AI

## 8. CLI Integration

- [x] 8.1 Implement `veriplan check <change-name>` subcommand using clap
- [x] 8.2 Implement `veriplan check --phase convertibility` (Phase 1 only, no model checking)
- [x] 8.3 Implement `veriplan bootstrap` subcommand that auto-configures openspec/config.yaml
- [x] 8.4 Wire up pipeline: parse → convertibility → translate → model check → annotate
- [x] 8.5 Implement `--format json` flag for machine-readable output
- [x] 8.6 Implement non-zero exit code on verification failure
- [x] 8.7 Implement exit code 2 for blocking convertibility issues

## 9. Dogfooding: veriplan against itself

- [x] 9.1 Run `veriplan check veriplan-plan-verifier --phase convertibility` → PASSES (finds 8 formalizable / 15 non-formalizable)
- [x] 9.2 Run `veriplan check veriplan-plan-verifier` → full pipeline runs end-to-end
- [x] 9.3 `veriplan bootstrap` auto-configures config.yaml with rules + context
- [ ] 9.4 Create a deliberately inconsistent spec and verify it's flagged as invalid
- [x] 9.5 Verify JSON output is parseable for both valid and invalid cases

## 10. Visualize: state-machine diagram of plan

- [x] 10.1 Modify `check` to write `.veriplan/results.json` cache (per-constraint pass/fail/timeout)
- [x] 10.2 Generate Mermaid flowchart: phase subgraphs + task nodes + constraint edges
- [x] 10.3 Generate DOT digraph (alternative format)
- [x] 10.4 Generate markdown table (fallback format)
- [x] 10.5 Add `veriplan visualize` CLI subcommand with `--format` and `-o` flags
- [x] 10.6 Overlay verification results on diagram (green/red edges)
- [x] 10.7 Test: `veriplan visualize veriplan-plan-verifier` produces valid Mermaid in stdout

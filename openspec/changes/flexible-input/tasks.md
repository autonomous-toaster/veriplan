# flexible-input

## Phase 1: Input Resolution

- [ ] 1.1 Add InputSource enum and InputResolver to CLI
- [ ] 1.2 Implement single-file input detection and parsing
- [ ] 1.3 Implement stdin input detection and parsing
- [ ] 1.4 Implement loose-directory input detection
- [ ] 1.5 Wire InputResolver into check and visualize subcommands

## Phase 2: Classification and Strictness

- [ ] 2.1 Add ConstraintCategory::PatternUngrounded variant
- [ ] 2.2 Split check_classifiability into pattern detection and grounding checks
- [ ] 2.3 Change MAY requirements from silent drop to INFO items
- [ ] 3.1 Add StrictnessProfile enum (Strict, Moderate, Lax)
- [ ] 3.2 Add --strict/--moderate/--lax CLI flags
- [ ] 3.3 Implement severity mapping per StrictnessProfile in checker
- [ ] 3.4 Make "no tasks"/"no requirements" severity depend on input mode

## Phase 3: Parser Flexibility

- [ ] 4.1 Add parse_content() function that tries both parsers on any content
- [ ] 4.2 Make PlanIR construction tolerant of empty tasks or empty requirements

## Phase 4: LSP Integration

- [ ] 5.1 Update LSP ChangeStore to handle single-file resolution
- [ ] 5.2 Add parse_content() support to LSP diagnostics

## Phase 5: Testing

- [ ] 6.1 Integration tests for single-file mode
- [ ] 6.2 Integration tests for stdin mode
- [ ] 6.3 Integration tests for strictness profiles
- [ ] 6.4 Dogfood: verify veriplan's own OpenSpec change passes

# flexible-input

## Phase 1: Input Resolution

- [x] 1.1 Add InputSource enum and InputResolver to CLI
- [x] 1.2 Implement single-file input detection and parsing
- [x] 1.3 Implement stdin input detection and parsing
- [x] 1.4 Implement loose-directory input detection
- [x] 1.5 Wire InputResolver into check and visualize subcommands

## Phase 2: Classification and Strictness

- [x] 2.1 Add ConstraintCategory::PatternUngrounded variant
- [x] 2.2 Split check_classifiability into pattern detection and grounding checks
- [x] 2.3 Change MAY requirements from silent drop to INFO items
- [x] 3.1 Add StrictnessProfile enum (Strict, Moderate, Lax)
- [x] 3.2 Add --strict/--moderate/--lax CLI flags
- [x] 3.3 Implement severity mapping per StrictnessProfile in checker
- [x] 3.4 Make "no tasks"/"no requirements" severity depend on input mode

## Phase 3: Parser Flexibility

- [x] 4.1 Add parse_content() function that tries both parsers on any content
- [x] 4.2 Make PlanIR construction tolerant of empty tasks or empty requirements

## Phase 4: LSP Integration

- [x] 5.1 Update LSP ChangeStore to handle single-file resolution
- [x] 5.2 Add parse_content() support to LSP diagnostics

## Phase 5: Testing

- [x] 6.1 Integration tests for single-file mode
- [x] 6.2 Integration tests for stdin mode
- [x] 6.3 Integration tests for strictness profiles
- [x] 6.4 Dogfood: verify veriplan's own OpenSpec change passes

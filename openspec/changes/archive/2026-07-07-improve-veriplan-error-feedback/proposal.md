## Why

Veriplan's error feedback sends AI models into 10-20 iteration loops on the same violation because error messages don't match the actual root cause. Analysis of 118 session logs (57MB) across two kobo projects reveals 5 distinct failure patterns, all stemming from one root cause: `classify()` scans the full requirement body text (including English prose paragraphs) for temporal keywords, producing false positive classifications. The model guesses at the cause, tries random rewordings, and wastes 15+ veriplan calls per violation.

## What Changes

- **Parser**: `extract_shall_statement()` currently returns the entire body text. Change it to return only the first paragraph (before first blank line or scenario heading). Body paragraphs are still stored for documentation but excluded from classification.
- **Grounding**: Add a pre-check that detects multi-keyword statements and emits a clear, actionable error: "GROUNDING AMBIGUITY: matches both BEFORE and ALWAYS. Split into separate requirements, one temporal keyword per requirement."
- **Pipeline**: If grounding failed (ambiguous or ungroundable), skip the BFS model checker for that requirement entirely. Prevents confusing downstream error messages.
- **Error messages**: Audit `suggest_fix()` in `bfs.rs` to reference actual detected keywords, not assumed ones. Replace generic hints with specific guidance.
- **CLI**: Add `--change` as an alias for the positional `[CHANGE]` argument.

## Capabilities

### New Capabilities

- `error-feedback`: Veriplan's error reporting pipeline — classification, grounding feedback, BFS violation messages, and CLI ergonomics. Covers all changes to how veriplan communicates violations to the user.

### Modified Capabilities

- *(none — this is a new capability, not modifying existing spec behavior)*

## Impact

- `src/parser/helpers.rs`: `extract_shall_statement()` — change to extract only first paragraph
- `src/translator/mod.rs`: `classify()` — no change needed (it already operates on `req.statement`; the fix is upstream in what gets stored as `statement`)
- `src/grounding/mod.rs`: `check_grounding()` — add multi-keyword pre-check
- `src/checker/convertibility.rs`: pipeline — skip BFS for requirements that failed grounding
- `src/checker/bfs.rs`: `suggest_fix()` — audit and improve error messages
- `src/cli.rs`: add `--change` alias
- `src/ir/mod.rs`: possibly add a field to track whether grounding failed

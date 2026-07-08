## Context

Veriplan's error feedback pipeline has a structural flaw: `extract_shall_statement()` in `src/parser/helpers.rs` returns the **entire body text** under a `### Requirement` heading, including English prose paragraphs. The `classify()` function in `src/translator/mod.rs` then scans this full text for temporal keywords (`" if "`, `"only one"`, `"after"`, `"fail"`+`"then"`, `"always"`, `"before"`). Body text keywords produce false positive classifications that cascade into confusing BFS model-checker error messages.

Analysis of 118 session logs (57MB) across two kobo projects shows this causes 10-20 iteration loops where the model guesses at root causes and tries random rewordings.

## Goals / Non-Goals

**Goals:**
- Fix `extract_shall_statement()` to return only the first paragraph (SHALL sentences), excluding body prose
- Add a grounding pre-check that detects multi-keyword statements with a clear error message
- Prevent BFS model checker from running on requirements that failed grounding
- Audit `suggest_fix()` messages to reference actual detected keywords
- Add `--change` CLI alias

**Non-Goals:**
- Not changing the `classify()` function itself (the fix is upstream in what gets stored as `statement`)
- Not changing the BFS model checker logic (only the error messages)
- Not changing the groundcontrol library (only veriplan's integration layer)

## Decisions

### D1: First-paragraph extraction instead of SHALL-sentence parsing

**Choice**: `extract_shall_statement()` returns the first paragraph (text before first blank line or `####` heading).

**Rationale**: The convention in working spec files is that the first paragraph contains all SHALL sentences, and subsequent paragraphs contain explanatory prose. Tree-sitter already parses paragraphs as separate nodes — we just need to stop after the first one. Full SHALL-sentence extraction would require NLP-level parsing and is unnecessary given the existing convention.

**Alternatives considered**:
- Regex-based SHALL extraction: Fragile, misses edge cases
- Full NLP parsing: Overkill for this problem
- Keeping current behavior: Proven to cause 10-20 iteration loops

### D2: Multi-keyword pre-check in grounding

**Choice**: After the grounder returns, check if multiple predicates matched with confidence > 0.5. If so, emit a `grounding_ambiguous_multi_keyword` blocker.

**Rationale**: The grounder already returns all candidates. We just need to check if more than one predicate has a match. This catches the problem before the BFS model checker runs.

### D3: Pipeline skip for grounding failures

**Choice**: If a requirement has `PatternUngrounded` status after grounding, skip it in the BFS model checker.

**Rationale**: Grounding failures mean the requirement can't be translated to LTL. Running the BFS checker on it produces confusing error messages that reference non-existent keywords.

### D4: Error message audit

**Choice**: Replace generic hints in `suggest_fix()` with messages that reference the actual detected keywords.

**Rationale**: Current messages say "remove AT MOST ONE / NOT CONCURRENTLY" even when the statement doesn't use those keywords. The fix is to include the detected keyword in the message.

## Risks / Trade-offs

- **[Risk] First-paragraph extraction may miss SHALL sentences in later paragraphs** → Mitigation: The convention is to put all SHALLs in the first paragraph. If a spec breaks this convention, the SHALLs in later paragraphs become ungrounded (caught by the grounding check).
- **[Risk] Multi-keyword pre-check may produce false positives for legitimate compound statements** → Mitigation: The pre-check only fires when multiple predicates match with confidence > 0.5. A compound statement like "T1.1 SHALL complete BEFORE T1.2 AND T1.3 SHALL run CONCURRENTLY" would trigger it, which is correct — such statements should be split.
- **[Trade-off] Not fixing `classify()` itself** → The function works correctly on clean input. The fix is upstream in the parser.

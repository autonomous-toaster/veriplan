# Proposal

## Why

A veriplan finding cannot be resolved from what veriplan prints.

The chevron-oauth-conformance run of 2026-11-20 shows the cost. One real finding — the ambiguous
pronoun in `keep the identifier it is given` — was printed as `prose_other ... (e.g. spec.md:24)`.
The change holds two files named `spec.md`, the pronoun sits on line 23, byte 70 of the file is
line 3, and the default line showed the suggestion instead of the message that named the two
candidate nouns. The agent read the pointer as a false positive, edited the wrong delta, committed
an unrelated reword, and filed a bug report that it later withdrew. Thirty-six tool results for one
finding.

Three defects sit behind that run, and each is measurable:

1. `Finding.file` is a basename. `file_name_str` keeps the last path component, so every delta
   in a change reports `spec.md`. `fix::resolve_path` then falls back to a recursive basename
   search and takes the first hit.
2. A scenario-step line drifts by one line for each preceding scenario in the file. Measured with
   v0.5.0: one scenario reports the correct line, two scenarios report +1, three report +2, four
   report +3. The parser joins the requirement body from blocks and then adds an anchor to an index
   in the joined text, so the arithmetic cannot survive the join.
3. `parse_step` hardcodes `start_byte: 0`, so step findings report snippet-relative byte offsets
   as file coordinates. The comment in `src/prose/mod.rs` claims this class was fixed; the sweep
   covered tasks and missed steps.

The published requirement *Finding coordinates are file-absolute* already forbids defect 3, and
*Preserve source locations* already forbids defect 2. Both were marked done in
2026-11-21-verdict-soundness. This change repairs the contract and closes the class that let a
correct finding read as a false positive.

## What Changes

- **Coordinates.** `Finding.file` becomes a change-relative path. A scenario step carries a byte
  span taken from the step's own block, and the line is derived from that span rather than from an
  index into the joined body. A requirement-body finding anchors at the body line, not the heading.
- **Identity.** `Finding` gains `element` (the requirement or scenario that owns the finding) and
  `excerpt` (the offending span). `ProseFinding` already computes both and the conversion discards
  them, so most of this is carrying an existing value.
- **Report shape.** Every kind gains an arity — plan-scoped or site-scoped. A group line becomes a
  header over listed members instead of a substitute for them: it names one true member and counts
  the rest behind an explicit remainder marker. Default output scales with the count — verdict at
  zero, full identity at one, one line per kind and remedy at several.
- **Vocabulary.** `prose_other` splits into per-rule kinds (`prose_pronoun_ambiguity`,
  `prose_passive_voice`, `prose_hedging`, `prose_one_instruction`, `prose_sentence_length`,
  `prose_synonym_inconsistency`, `prose_slop_word`). `prose_other` stays as the fallback for an
  unmapped rule. No alias shim: the value has been in use for two changes and carries seven
  unrelated rules under one name.
- **Coherence.** `advisory` means "no model formula backs this finding", not "this finding does not
  block". A strict prose blocker stays advisory and still blocks the verdict.
- **No change** to the diagnosis rules themselves, to the model checker, or to the JSON top-level
  keys. `findings[]` keeps every field it has today.

## Capabilities

### New Capabilities

None. The defect is drift inside existing capabilities, not a missing one.

### Modified Capabilities

- `output-contract`: coordinates carry a path; a group line stops substituting for its members;
  `kind` names the prose rule; a finding carries its element and its excerpt; `advisory` is tied
  to the model rather than to severity.
- `plan-parser`: a scenario step carries a byte span from its own block, and the step line is
  derived from that span; a requirement-body source reports the body line.

## Impact

- Code: `src/ir/mod.rs` and `src/ir/kind.rs` (finding shape, arity table), `src/parser/mod.rs`
  and `src/parser/helpers.rs` (paths, spans, body anchor), `src/prose/mod.rs` (kind mapping,
  element and excerpt), `src/annotator/mod.rs` (conversion, grouping, rendering), `src/fix.rs`
  (path resolution).
- Consumers: the JSON `findings[]` array gains two fields and renames seven `kind` values. The
  LSP diagnostic range becomes correct because the spans become correct.
- Not affected: the model checker, the convertibility checks, `openspec validate` behaviour.
- Reproduction to re-run at the end: the chevron-oauth-conformance change at `b80c166^` must
  report `specs/chevron-cli/spec.md:23` with the quoted span.

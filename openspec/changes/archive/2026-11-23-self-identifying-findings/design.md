# Design

## Context

A finding is produced three ways: a convertibility `CheckItem`, an annotated model-check
violation, and a prose finding from steve. All three land in `Finding` (`src/ir/mod.rs`) and are
rendered by `src/annotator/mod.rs`. The rendered text is the product; an agent reads it and edits
a file. Measured against the chevron-oauth-conformance run of 2026-11-20, every field that makes a
finding resolvable was either wrong, ambiguous, or discarded.

## D1 — `file` is a change-relative path

`helpers::file_name_str` returns the last path component, so every spec delta in a change reports
`spec.md`. The oauth change has two; chevron-backbone has six.

`Finding.file` becomes the path relative to the change directory (`specs/chevron-cli/spec.md`).
The parser already receives the full path at `parse_spec_file`; only the last step throws the
prefix away.

Consequences:
- A location string is unique inside the change, which is the scope a finding is actionable in.
- `fix::resolve_path` joins against the change directory and succeeds on the first branch, so the
  recursive basename search (`find_file_by_name`, first hit) stops being the common path. That
  search made an auto-applied edit able to land in the wrong delta; the path fix removes the risk
  without touching the resolver.
- Single-file and stdin input keep the path they were given, because no change directory exists.

Rejected: an absolute path. The change directory moves between machines and checkouts, and the
printed string is also read by a human.

## D2 — a step span comes from the step's own block

The parser builds the requirement body by concatenating the sibling block nodes with a single
newline (`src/parser/mod.rs`), records `body_start_line` from the first block, and then computes
each step line as `body_start_line + i` where `i` is the index in the joined text. That mapping is
only valid if the joined text preserves the file's line structure, and the join does not: blank
lines between blocks are replaced by the separator.

Measured drift, v0.5.0, one spec file, pronoun in the last scenario:

| scenarios in file | actual line | reported line |
|---|---|---|
| 1 | 11 | 11 |
| 2 | 15 | 16 |
| 3 | 19 | 21 |
| 4 | 23 | 26 |

A ten-scenario delta reports its last step nine lines off, which lands on the next scenario's text.

Fix: `extract_scenarios` already takes a `bytes` parameter and ignores it (`_bytes`). Record each
step's span from the block that produced the step, then derive the line from the span. Line
arithmetic over the joined text disappears rather than being corrected.

The same argument applies to `start_byte: 0` in `parse_step`: the field exists and is always zero,
so a step finding reports a snippet-relative offset as a file coordinate.

## D3 — a requirement-body source is the body line

`Requirement.source.start_line` is the heading line, so a prose finding in a body paragraph
reports the `### Requirement:` heading. Measured: heading 12, body 14, reported 12. The heading is
a real line, so this is not a broken offset — it is the wrong anchor. The body line is the one that
holds the offending text.

## D4 — `Finding` carries `element` and `excerpt`

`ProseFinding` already computes `element` (`Scenario 'A client registers itself' (Then step)`) and
`snippet`, and `check_item_to_finding` drops both. The formatter cannot re-derive either: it has
no source text and no plan.

- `element: String` — the owner of the finding, empty when the finding is plan-scoped.
- `excerpt: Option<String>` — the offending span with a few words of context, filled where the text
  is in hand.

Both are filled at construction. The formatter only renders them, so the shape stays serialisable
and the JSON carries what the human text carries.

## D5 — arity, and grouping as summarisation

`group_by_kind` compares `kind` alone, keeps the first member, and prints that member's location
for the whole group. That is a substitution: the other members leave the output. It is correct for
`no_tasks` (one fact about the plan) and wrong for `prose_pronoun_ambiguity` (N sentences, N edits).

`KIND_NAMES` (`src/ir/kind.rs`) gains an arity: `Plan` or `Site`. The formatter cannot apply a
plan summary to a site fact, so the mistake becomes unrepresentable rather than merely discouraged.

The rule that replaces `group_by_kind`:

> A group line names one true member of the group. Any other member is counted, never described by
> that member's location or remedy.

## D6 — density scales with the count

The agent is the primary reader and its context is the budget, so default output stays short. The
shortness comes from counting, not from hiding:

| findings | default output |
|---|---|
| 0 | verdict and the constraint split (unchanged) |
| 1 | three lines: severity, kind and pointer; element and reason; quoted span |
| N | one line per kind and remedy, one true member's pointer, `+N more (--verbose)` |

`--verbose` lists every finding, one line each, with element and pointer. `--format json` already
carries everything and keeps doing so.

Rejected: listing every site by default. It is honest but it spends the reader's context on
findings the reader can enumerate with one call. The remainder marker makes the omission explicit,
which is the property that was missing.

## D7 — `kind` names the prose rule

`Kind` has `ProseSlopWord` and `ProseOther` for seven curated steve rules, so a pronoun ambiguity
reports as `prose_other`. `KIND_NAMES` gains one entry per curated rule, and `ProseOther` stays as
the fallback for a rule without an entry.

No alias: `kind_of` accepts the aliases in the table, and none is added for the old spellings. The
value changed two changes ago and seven rules under one name is the defect.

## D8 — `advisory` describes the model, not the consequence

Today a strict prose finding carries `advisory: true` and `severity: blocker`, and the verdict
reads `VALID (with blockers)`. Three signals disagree, and an agent that notices files a bug
report — which is what happened.

`advisory` becomes "no model formula backs this finding". Severity keeps its meaning and the
verdict keeps deriving from severity. A strict prose blocker is advisory and blocking, and the
output says so without contradiction.

## Observed while validating: AFTER is reversed

The spec bodies in this change use BEFORE for every ordering. That is a workaround, not a
preference. A plan that satisfies `T1.1 SHALL complete BEFORE T2.1 SHALL complete` fails the same
ordering written as `T2.1 SHALL complete AFTER T1.1 SHALL complete` (v0.5.0, two phases, one task
each: VALID against one violation_sequential). The model chains tasks in phase order, so the AFTER
translation is reversed or the cited tasks are swapped. Recorded as kata issue `zhdt`.

## Out of scope

- The diagnosis rules themselves, the checker backends, and the JSON top-level keys.
- `--fix` eligibility. Measured: a `SlopWord` finding reaches `check` as `structural`, so
  `--fix` declines to apply it. That is a separate defect with its own evidence and it does not
  depend on this change.
- An alias or deprecation path for the renamed kinds.

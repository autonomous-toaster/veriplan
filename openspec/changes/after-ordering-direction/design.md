# Design

## Context

`classify` sends a body holding ` before ` or ` after ` to `SequentialOrder`
(`src/translator/mod.rs`, `is_sequential`). The arm in `src/translator/ltl.rs` then builds
`[](active_<later> -> done_<earlier>)`.

The direction of "earlier" is decided twice in that arm, and both decisions are wrong for AFTER.

## D1 — the pair finder works on the sentence the product produces

The three patterns are `"{id} before"`, `"after {id}"` and `"{id} complete"`, matched against the
lowercased statement. A task id in a requirement body carries a `T` prefix, and an RFC 2119 verb
stands between the id and the keyword: `t1.1 shall complete before t2.1`. None of the patterns
appears in that string, so the finder returns `None` for every real body.

The fix is a pattern that describes the grammar rather than a test sentence: a task id token, an
optional verb phrase, then the keyword. The id has to be matched on a whole-identifier boundary,
which `util::mentions_task` and `util::find_task` already do; the pair finder should reuse one of
them instead of matching `"1.1"` inside `"t1.1"`.

## D2 — the pair is (earlier, later), not (other, id)

The caller unpacks `Some((before_id, after_id))`. For BEFORE the arm returns `(id, other)`, which
is correct. For AFTER the arm returns `(other, id)`, which is the reverse: in
`T2.1 SHALL complete AFTER T1.1`, the earlier task is the one after the keyword.

Both arms SHALL return the same tuple order. The direction belongs to the keyword, and the tuple is
always read the same way.

## D3 — the fallback takes the direction from the keyword

When the finder cannot name a pair, the arm falls back to `refs[0]` and `refs[1]`, in statement
order. That is a guess, and the guess is only right for BEFORE.

The fallback SHALL keep statement order when the body contains ` before `, and SHALL reverse the
order when the body contains ` after `. A body that carries neither SHALL NOT reach the arm
(`classify` already guarantees that), so the fallback needs no third branch.

Rejected: delete the fallback outright. A body with three cited ids has no single pair, and the
fallback still names the first two in the direction the keyword states. Deleting the fallback would
turn those bodies into unmodelled constraints, which changes counts for plans that are correct
today.

## D4 — the tests use the grammar

`test_find_sequential_pair_before` and `test_find_sequential_pair_after` use `"1.1 before 1.2"`
and `"1.2 after 1.1"`. Neither is a requirement body: the grammar requires the `T` prefix and an
RFC 2119 keyword. The tests therefore exercise a shape no caller produces, which is why the dead
patterns and the reversed arm both survived a green suite.

The two tests SHALL be rewritten against grammar-shaped bodies. A third test SHALL assert the
property that was missing: the BEFORE body and the AFTER twin describe one ordering, so the two
bodies SHALL produce the same formula.

## D5 — the fixture is the regression

The reproduction is three runs over one plan: BEFORE with a forward chain, AFTER with a forward
chain, AFTER with a reversed chain. A unit test covers the formula; a fixture covers the verdict,
because the reversal was invisible until the verdict disagreed with the sentence.

## Out of scope

- The `AFTER` predicate slots in `grounding/signature.rs` (named `earlier`/`later`). The slots
  feed the grounding status only; `generate_ltl` never receives them, so they cannot influence the
  formula either way.
- `if_then`, `concurrently` and the exclusive arm. No evidence of a direction defect there.
- Any alias or compatibility path for a plan that today passes with a reversed AFTER chain. Such a
  plan states one thing and models another, so the change is a correction rather than a break.

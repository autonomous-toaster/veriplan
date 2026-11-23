# Proposal

## Why

An ordering constraint written with AFTER is translated backwards, so a correct plan is reported as
unsound.

Measured on v0.5.0 with two phases and one task each:

| body | phase chain | expected | observed |
|---|---|---|---|
| `T1.1 SHALL complete BEFORE T2.1 SHALL complete` | 1.1 then 2.1 | VALID | VALID |
| `T2.1 SHALL complete AFTER T1.1 SHALL complete` | 1.1 then 2.1 | VALID | INVALID |
| `T2.1 SHALL complete AFTER T1.1 SHALL complete` | 2.1 then 1.1 | INVALID | VALID |

The third row settles the direction: the same sentence passes only when the model runs the tasks
backwards.

Two defects sit in the sequential arm of `src/translator/ltl.rs`:

1. `find_sequential_pair` matches `"1.1 before"`, `"after 1.1"` and `"1.1 complete"` — a bare id,
   with no `T` prefix and no verb between the id and the keyword. A real body reads
   `T1.1 SHALL complete BEFORE T2.1 SHALL complete`, so all three patterns miss, the function
   returns `None`, and the positional fallback decides. The fallback assumes statement position
   equals execution order, which is true for BEFORE and reversed for AFTER.
2. The `AFTER` arm that does match returns the pair as `(other, id)`, and the caller reads
   `(before, after)`. The arm is reversed as well. `translator/tests.rs` asserts the reversed value
   on `"1.2 after 1.1"`, with a comment that contradicts the assertion, so the test locks the bug
   in. Both unit tests use sentences the grammar forbids, which is why dead code with a wrong
   direction survived.

The consequence is not only a false violation. The natural repair for an author is to rewrite the
requirement into the direction the tool happens to accept, which silently inverts the plan.

## What Changes

- `find_sequential_pair` SHALL match a body that cites `T`-prefixed ids with a verb between the id
  and the keyword, and SHALL return `(earlier, later)` for both BEFORE and AFTER.
- The positional fallback SHALL take the direction from the keyword, not from statement position.
  A body that carries no sequential keyword SHALL NOT reach the sequential arm.
- The two unit tests SHALL use requirement bodies the grammar permits, and a new test SHALL assert
  that a BEFORE body and the AFTER twin produce one formula.
- The published requirement *Generate LTL for sequential order* states "the earlier task taken from
  statement position", which is the defect written into the spec. The delta replaces that clause.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rule-translator`: the earlier task comes from the keyword; the pair finder matches a real body;
  the positional fallback follows the keyword; the two twins translate alike.

## Impact

- Code: `src/translator/ltl.rs` (pair finder, fallback), `src/translator/tests.rs` (two tests
  replaced, one added).
- Behaviour: a plan whose AFTER constraints were previously reported as violations will now pass,
  and a plan whose AFTER constraints were previously satisfied by a reversed chain will now fail.
  Both are the intended meaning of the sentence. No other category is touched.
- Not affected: the checkers, the Promela model, the grounding library, the finding shape.
- The `self-identifying-findings` change writes every ordering as BEFORE as a workaround; this
  change removes the reason for the workaround, and that change does not need to change.

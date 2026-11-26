# Tasks

## 1. Tally what the model carries

- [x] 1.1 Tally the modelled constraints in `check_diversity` instead of the classified requirement text. Verify that a plan of three sequential requirements still reports `low_diversity` and that appending an unmodelled conditional leaves the INFO standing.
- [x] 1.2 Name the argument that a non-sequential category is missing. Verify that exclusive, conditional and concurrent each report their own message and that the generic message remains for a body with no keyword.
- [x] 1.3 Rewrite the `low_diversity` hint to state that a non-sequential constraint cites two task ids and that the line is advisory. Verify the hint carries a shape that cites two task ids.
- [x] 1.4 Pin both defects in tests. Verify each new test fails against the old tally and the old message.

## 2. Land

- [x] 2.1 Run `openspec validate diversity-counts-modelled --strict`, `veriplan check` on this change, and the Justfile gate. Verify each reports no warning and no failure.

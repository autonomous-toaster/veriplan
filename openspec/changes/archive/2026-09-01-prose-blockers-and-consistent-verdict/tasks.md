## 1. Verdict Consistency

- [x] 1.1 Derive `status_label` from the flattened `findings[]` set so the status reflects any `blocker`-severity finding
- [x] 1.2 Make the exit code reflect prose blockers in Strict mode (non-zero when a prose blocker is present)

## 2. Severity Bucketing

- [x] 2.1 Route a `blocker`-severity prose finding into `report.blockers` instead of `report.info` in `verify_with_strictness`

## 3. Prose Blockers in Strict

- [x] 3.1 Map `OneInstructionPerSentence` and `PronounAmbiguity` to severity `blocker` in Strict mode
- [x] 3.2 Keep prose findings advisory (warning/info) in Moderate and Lax modes

## 4. File-Absolute Coordinates

- [x] 4.1 Compute each prose snippet's start offset in its source file from the element's source span
- [x] 4.2 Add the snippet start offset to the reported `line`, `start`, and `end` on prose findings
- [x] 4.3 Document the file-absolute coordinate contract in the `output-contract` spec

## 5. Pre-commit

- [x] 5.1 Ensure `--pre-commit` mode treats a Strict prose blocker as exit-1, consistent with other blockers

## 6. Validation

- [x] 6.1 Add a test asserting the verdict is not "✓ VALID" when a `blocker`-severity prose finding is present
- [x] 6.2 Add a test asserting a task-description prose finding reports its real line and file byte offsets
- [x] 6.3 Run `veriplan check prose-blockers-and-consistent-verdict` (expecting zero blockers)

# Tasks

## 1. Emit the failure block per task

- [x] 1.1 Add the atom walk to \`LtlCondition\` and \`LtlFormula\` and the per-task failure-flag predicate. Verify the unit test reaches every variant and the test showing that the flag of task 1.10 does not name task 1.1.
- [x] 1.2 Emit the failure flag and the branch only for the tasks a claim names. Verify a three-task model carries one flag, the branch under that flag, and no other flag.
- [x] 1.3 Pin the state count on a closing probe and show that a refuting claim is not a count probe. Verify the twenty-four-task to eight-task ratio stays under four and the two claims report the same count.

## 2. Report a stopped search by its cause and its cost

- [x] 2.1 Give the unchecked outcome a cause and report the time allowed and the time used. Verify the reason never names a budget that \`pan\` does not have and that a claim over the allowance prints both times.
- [x] 2.2 Carry the states used and the states allowed into the constraint summary and print both. Verify a stopped claim reports a used value no greater than the allowed one in the human report and in \`--format json\`.
- [x] 2.3 Report the share given to each claim and refuse a budget that cannot be honoured. Verify \`VERIPLAN_MAX_STATES=0\` and \`VERIPLAN_MAX_STATES=abc\` are reported instead of replaced.

## 3. Do not score a stopped backend as agreement

- [x] 3.1 Count a claim either backend left undecided outside the match tally. Verify a comparison holding one undecided claim states one not compared instead of full agreement.
- [x] 3.2 Name the backend that stopped, the claim that stopped, and the cause. Verify \`--compare\` names both when one side stops at the budget.

## 4. Pin the in-process engine

- [x] 4.1 Assert the state count and the elapsed time of the in-process engine on a fixed twenty-task chain. Verify the test passes at the measured count and prints both numbers.
- [x] 4.2 Fail the gate on the exponential shape. Verify a model carrying a branch on every task fails and names the task count at which the count stopped growing linearly.
- [x] 4.3 Measure the liveness path on the same chain. Record the exact product per task count. Verify the test compares the count with the invisible bit against the count without the invisible bit. Verify the test fails on a change in either direction.
- [x] 4.4 Measure a claim that cannot see a flag some other claim names. Record the exact product per task count. Verify the fixture carries two flags and one claim naming neither.

## 5. Land

- [x] 5.1 Run \`openspec validate model-check-throughput --strict\` and the file-length gate. Verify the validation reports no warning and no touched file exceeds 550 lines.
- [x] 5.2 Run \`veriplan check\` on this change with both backends. Verify both report the same verdict, the same violated count and the modelled split.
- [x] 5.3 Re-measure the corpus scale for both backends and record the requirement left unsatisfied. Verify every number comes from a fresh run naming the model shape and the backend, and the record names the requirement and the engine issue.

## Workflow follow-up

- Archive the change once the project's review requirements are satisfied.
- Verify the archived spec carries the added requirements and no duplicate of the existing unchecked-constraint requirement.

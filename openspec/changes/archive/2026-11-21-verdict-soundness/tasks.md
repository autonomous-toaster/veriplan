## 1. Translator Correctness

- [x] 1.1 Test sequential order ahead of fixed time in `classify`
- [x] 1.2 Narrow `is_time_ref` to a clock or duration literal
- [x] 1.3 Add a seam test over `classify` and `generate_ltl` with a task-citing statement
- [x] 1.4 Remove the always-true placeholder from the fixed-time and global arms
- [x] 1.5 Reject a temporal relation between one task and the same task
- [x] 1.6 Report a requirement body whose second constraint clause was not modelled

## 2. Verdict Truthfulness

- [x] 2.1 Mark each translated constraint as modelled or unmodelled
- [x] 2.2 Exclude an unmodelled constraint from the satisfied count
- [x] 2.3 Name the checker backend in the human verdict
- [x] 2.4 Add the backend and the modelling split to the JSON output
- [x] 2.5 Fail a search that explores no transitions on a non-empty model
- [x] 2.6 Add the calibration canary to the generated model set
- [x] 2.7 Enforce a search budget on every backend

## 3. Coordinates

- [x] 3.1 Compute scenario-step sources against the file
- [x] 3.2 Sweep every remaining snippet-relative source constructor
- [x] 3.3 Add coordinate tests for a step finding and a prose finding

## 4. Baseline Grounding

- [x] 4.3 Document the archive policy in the design artifact
- [x] 4.4 Relocate the init task table rule into the design artifact
- [x] 4.5 Stop treating one word of a task description as an alias

  Building the signature from archived task lists, and naming the change that owns a cited task
  id, are not in this change. They need the project root threaded from `cmd_check` through
  `check_convertibility` into `check_grounding`, and they are tracked as their own change so
  the published spec does not claim behaviour that does not exist.

## 5. Validation

- [x] 5.1 Run `openspec validate verdict-soundness --strict`
- [x] 5.2 Run `veriplan check verdict-soundness --strict`
- [x] 5.4 Record the model-scale measurement that makes 5.2 report UNKNOWN
- [x] 5.3 Re-check the downstream corpus and record the violations found

  The 62-requirement corpus reports VALID with no grounding findings at the pinned dependencies,
  and the change's own plan is VALID at convertibility. The model-level re-check is blocked by
  the model-shape defect tracked separately.

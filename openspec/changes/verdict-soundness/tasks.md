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

- [ ] 4.1 Build the grounding signature from archived task lists
- [ ] 4.2 Report a cited task id owned by an archived change
- [x] 4.3 Document the archive policy in the grounding spec
- [x] 4.4 Relocate the init task table rule into the design artifact
- [x] 4.5 Stop treating one word of a task description as an alias

  Deferred. Both need the project root threaded from `cmd_check` through `check_convertibility`
  into `check_grounding`, which touches five signatures and every call site. The half that
  unblocks authors already lands: `absent_identifiers` names the cited id and the finding no
  longer asks for an id reference. The owning-change half is a separate reviewable change.

## 5. Validation

- [x] 5.1 Run `openspec validate verdict-soundness --strict`
- [x] 5.2 Run `veriplan check verdict-soundness --strict`
- [x] 5.4 Record the model-scale measurement that makes 5.2 report UNKNOWN
- [ ] 5.3 Re-check the downstream corpus and record the violations found

  Pending: the corpus needs both dependencies tagged to be a reproducible measurement. See the
  kata record for the numbers taken against the working trees.

## 1. Suppress Aggregate Diagnostics

^ - [x] 1.1 Remove the `task_count` info emission from `checker/checks.rs`
^ - [x] 1.2 Remove the `scenario_count` info emission from `checker/checks.rs`
^ - [x] 1.3 Remove the `constraint_diversity` info emission from `checker/checks.rs`
^ - [x] 1.4 Remove the `classification_summary` info emission from `checker/checks/classifiability.rs`
^ - [x] 1.5 Remove the `grounding_summary` info emission from `grounding/mod.rs`
^ - [x] 1.6 Add the scenario count to the verbose plan summary in `annotator/mod.rs`
^ - [x] 1.7 Add a test asserting `findings[]` has no aggregate `prose_other` summaries
^ - [x] 1.8 Run `veriplan check` on the change, expecting VALID with no blockers

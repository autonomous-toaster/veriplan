## 1. Add tests for merge_config

- [x] 1.1 Add test for fresh file creation (merge_config creates file with schema + veriplan content)
- [x] 1.2 Add test for idempotent re-run (second run produces identical output)
- [x] 1.3 Add test for existing content preservation (original context preserved, new content added)
- [x] 1.4 Add test for rules content (all rule categories present, no tool self-reference in context)

## 2. Add tests for yaml_merge

- [x] 2.1 Add test for context append (new context appended to existing with separator)
- [x] 2.2 Add test for rule dedup (duplicate rules not added, new rules added)

## 3. Verify

- [x] 3.1 Run `cargo test` and verify all tests pass

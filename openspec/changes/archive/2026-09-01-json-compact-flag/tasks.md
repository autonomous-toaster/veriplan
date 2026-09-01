## 1. CLI flags

- [x] 1.1 Add `--json` flag to `Check` command struct in `src/main.rs`
- [x] 1.2 Add `--compact` flag to `Check` command struct in `src/main.rs`
- [x] 1.3 Wire `--json` and `--compact` into `run_check()` format resolution

## 2. JSON output fixes

- [x] 2.1 Remove `verbose &&` gate on `convertibility_report` in `format_json()` — always include report
- [x] 2.2 Update test `test_format_json_not_verbose_excludes_report` to expect report included

## 3. Compact JSON output

- [x] 3.1 Add `compact` parameter to `format_json()` — minified vs pretty output
- [x] 3.2 Wire compact mode into format dispatch in `main.rs` and `cli.rs`

## 4. Human output summary

- [x] 4.1 Add warning/info summary counts to default `format_human()` output (after blockers)
- [x] 4.2 Group warnings by `check` type for compact summary description
- [x] 4.3 Keep `--verbose` output unchanged (full warning/info expansion)

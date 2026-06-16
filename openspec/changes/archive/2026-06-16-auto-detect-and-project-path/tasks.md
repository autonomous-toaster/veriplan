## 1. CLI: Core

- [x] 1.1 Change `change` field in `Commands::Check` from `String` to `Option<String>`
- [x] 1.2 Add `--format` argument with default value `"openspec"` and validate against known formats
- [x] 1.3 Implement no-arg mode: when no change is given, scan `openspec/changes/` in CWD, exclude `archive/`, collect all active change directories

## 2. CLI: Disambiguation

- [x] 2.1 Implement disambiguation logic: try as change name first; fall back to directory path only if change doesn't exist AND argument looks like a path

## 3. CLI: Directory path fallback

- [x] 3.1 Implement directory-path mode: when argument is a path (not a change name), resolve openspec relative to that path instead of CWD

## 4. Parser: Project-root-aware change discovery

- [x] 4.1 Add `discover_changes(project_root: &Path) -> Vec<String>` that finds all non-archived changes in `project_root/openspec/changes/`
- [x] 4.2 Add `is_archive_dir(name: &str) -> bool` helper to filter out `archive/`
- [x] 4.3 Refactor `locate_change` to accept an optional project root parameter (defaults to CWD)
  (Note: resolution moved to main.rs find_change_dir; locate_change unchanged)

## 5. Checker: Multi-change verification

- [x] 5.1 Add `verify_all(plans: &[(String, PlanIR)], ...)` that runs `verify` on each plan and collects results
- [x] 5.2 Add `VerificationResult::merge(results: Vec<VerificationResult>)` that produces a combined report

## 6. Annotator: Multi-change output

- [x] 6.1 Update `format_human` to handle combined multi-change results (print per-change sections)
- [x] 6.2 Update `format_json` to output an array of per-change results under a `changes` key

## 7. Format extensibility scaffold

- [x] 7.1 Add `Format` enum with one variant `Openspec` and a `from_str` that validates
- [x] 7.2 Thread `Format` through `run_check` → parser selection: for now, one match arm that calls the openspec parser
- [x] 7.3 Error on unknown format: `veriplan check --format speckit` → "unknown format 'speckit'. Supported: openspec"

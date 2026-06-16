## 1. CLI: Core

- [ ] 1.1 Change `change` field in `Commands::Check` from `String` to `Option<String>`
- [ ] 1.2 Add `--format` argument with default value `"openspec"` and validate against known formats
- [ ] 1.3 Implement no-arg mode: when no change is given, scan `openspec/changes/` in CWD, exclude `archive/`, collect all active change directories

## 2. CLI: Disambiguation

- [ ] 2.1 Implement disambiguation logic: try as change name first; fall back to directory path only if change doesn't exist AND argument looks like a path

## 3. CLI: Directory path fallback

- [ ] 3.1 Implement directory-path mode: when argument is a path (not a change name), resolve openspec relative to that path instead of CWD

## 4. Parser: Project-root-aware change discovery

- [ ] 4.1 Add `discover_changes(project_root: &Path) -> Vec<String>` that finds all non-archived changes in `project_root/openspec/changes/`
- [ ] 4.2 Add `is_archive_dir(name: &str) -> bool` helper to filter out `archive/`
- [ ] 4.3 Refactor `locate_change` to accept an optional project root parameter (defaults to CWD)

## 5. Checker: Multi-change verification

- [ ] 5.1 Add `verify_all(plans: &[(String, PlanIR)], ...)` that runs `verify` on each plan and collects results
- [ ] 5.2 Add `VerificationResult::merge(results: Vec<VerificationResult>)` that produces a combined report

## 6. Annotator: Multi-change output

- [ ] 6.1 Update `format_human` to handle combined multi-change results (print per-change sections)
- [ ] 6.2 Update `format_json` to output an array of per-change results under a `changes` key

## 7. Format extensibility scaffold

- [ ] 7.1 Add `Format` enum with one variant `Openspec` and a `from_str` that validates
- [ ] 7.2 Thread `Format` through `run_check` → parser selection: for now, one match arm that calls the openspec parser
- [ ] 7.3 Error on unknown format: `veriplan check --format speckit` → "unknown format 'speckit'. Supported: openspec"

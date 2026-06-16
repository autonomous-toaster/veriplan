## ADDED Requirements

### Requirement: Discover active changes

The change directory scanner (T2.1) SHALL complete BEFORE the multi-change verifier (T3.1) SHALL run.

#### Scenario: No-arg check on project with active changes
- **WHEN** T1.3 (no-arg mode) detects `openspec/changes/` with two active changes and one archived change
- **THEN** T2.1 SHALL discover both active changes
- **THEN** T3.1 SHALL verify both active changes and skip the archived one

#### Scenario: No-arg check on project without openspec
- **WHEN** T1.3 (no-arg mode) detects no `openspec/changes/` directory
- **THEN** T1.3 SHALL exit with a clear error message

#### Scenario: Combined report format
- **WHEN** T3.1 (multi-change verification) completes
- **THEN** T4.1 SHALL produce a per-change section in human-readable output
- **THEN** T4.2 SHALL output an array of per-change results in JSON

### Requirement: Ignore archived changes

T2.2 (archive filter) SHALL ALWAYS exclude `archive/` from the change discovery result.

#### Scenario: Archived change is skipped
- **WHEN** T2.1 scans `openspec/changes/` and finds `archive/my-old-change/`
- **THEN** T2.2 SHALL filter out `my-old-change`
- **THEN** T3.1 SHALL NOT attempt to verify it

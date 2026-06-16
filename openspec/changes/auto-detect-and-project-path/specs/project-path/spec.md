## ADDED Requirements

### Requirement: Accept project path argument

The directory path resolver (T1.4) SHALL complete BEFORE the change directory scanner (T2.1) SHALL run.

#### Scenario: Check external project path
- **WHEN** T1.4 resolves `/path/to/other-project`
- **AND** `/path/to/other-project/openspec/changes/` exists with active changes
- **THEN** T2.1 SHALL scan the external project's changes
- **THEN** T3.1 SHALL verify all active changes in the remote project

#### Scenario: Check path without openspec
- **WHEN** T1.4 resolves `/path/to/plain-project`
- **AND** `/path/to/plain-project/` has no `openspec/` directory
- **THEN** T1.4 SHALL exit with a clear error

### Requirement: Disambiguate change name from path

IF T1.5 (disambiguation) determines the argument is a known change name, THEN the classic change-based resolver SHALL run. IF the change name is not found AND the argument looks like a path, THEN T1.4 (directory path resolver) SHALL run.

#### Scenario: Change name takes priority
- **WHEN** T1.5 receives `my-change`
- **AND** `openspec/changes/my-change/` exists
- **THEN** T1.5 SHALL use the classic change-based resolver
- **THEN** T1.4 SHALL NOT run

#### Scenario: Ambiguous path triggers fallback
- **WHEN** T1.5 receives `../other-project`
- **AND** there is no change named `../other-project`
- **THEN** T1.5 SHALL invoke T1.4 to resolve as a directory path

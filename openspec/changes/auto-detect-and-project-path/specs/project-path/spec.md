## ADDED Requirements

### Requirement: Accept project path argument

The directory path resolver (T3.1) SHALL complete BEFORE the change directory scanner (T4.1) SHALL run.

#### Scenario: Check external project path
- **WHEN** T3.1 resolves `/path/to/other-project`
- **AND** `/path/to/other-project/openspec/changes/` exists with active changes
- **THEN** T4.1 SHALL scan the external project's changes
- **THEN** T5.1 SHALL verify all active changes in the remote project

#### Scenario: Check path without openspec
- **WHEN** T3.1 resolves `/path/to/plain-project`
- **AND** `/path/to/plain-project/` has no `openspec/` directory
- **THEN** T3.1 SHALL exit with a clear error

### Requirement: Disambiguate change name from path

T2.1 (disambiguation) SHALL complete BEFORE T3.1 (directory path resolver) SHALL run.

#### Scenario: Change name takes priority
- **WHEN** T2.1 receives `my-change`
- **AND** `openspec/changes/my-change/` exists
- **THEN** T2.1 SHALL use the classic change-based resolver
- **THEN** T3.1 SHALL NOT run

#### Scenario: Ambiguous path triggers fallback
- **WHEN** T2.1 receives `../other-project`
- **AND** there is no change named `../other-project`
- **THEN** T2.1 SHALL invoke T3.1 to resolve as a directory path

## ADDED Requirements

### Requirement: Format selection flag

The format validator (T7.1) SHALL complete BEFORE the parser selection (T7.2) SHALL use the selected format.

#### Scenario: Default format is openspec
- **WHEN** T7.1 receives no `--format` argument
- **THEN** T7.2 SHALL select the openspec parser
- **THEN** the check SHALL behave identically to current behavior

#### Scenario: Explicit openspec format
- **WHEN** T7.1 receives `--format openspec`
- **THEN** T7.2 SHALL select the openspec parser
- **THEN** the check SHALL behave identically to default mode

#### Scenario: Invalid format name
- **WHEN** T7.1 receives `--format speckit`
- **THEN** T7.3 SHALL exit with an error listing the supported formats (`openspec`)

### Requirement: Format registry pattern

T7.2 (parser selection) SHALL ALWAYS use a single match expression. No plugin system, trait, or dynamic loading SHALL be introduced until a second format is actually implemented.

#### Scenario: Format selection is a single match
- **WHEN** T7.2 processes `--format openspec`
- **THEN** T7.2 SHALL use one match arm to select the openspec parser
- **THEN** T7.2 SHALL NOT use a trait implementation or dynamic loading

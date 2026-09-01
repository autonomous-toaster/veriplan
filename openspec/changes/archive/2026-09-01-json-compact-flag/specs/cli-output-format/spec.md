## ADDED Requirements

### Requirement: JSON output includes convertibility report
`veriplan check --json` SHALL always include the `convertibility_report` field in its JSON output, regardless of the `--verbose` flag.

#### Scenario: JSON output with blockers
- **WHEN** `veriplan check --json` runs on a plan with convertibility blockers
- **THEN** the JSON output SHALL contain a `convertibility_report` field with the blockers array populated

#### Scenario: JSON output without verbose
- **WHEN** `veriplan check --json` runs without `--verbose`
- **THEN** the JSON output SHALL contain a `convertibility_report` field

### Requirement: Compact JSON output mode
`veriplan check --json --compact` SHALL produce minified JSON with short field names and warnings/info as counts instead of arrays.

#### Scenario: Compact JSON has short keys
- **WHEN** `veriplan check --json --compact` runs
- **THEN** the output SHALL use `plan` instead of `plan_name`, `ok` instead of `convertible`, `blk` instead of `blockers`

#### Scenario: Compact JSON has warning count
- **WHEN** `veriplan check --json --compact` runs on a plan with warnings
- **THEN** the output SHALL contain a `warn` field with the count as a number, not an array

#### Scenario: Compact JSON is parseable by toon
- **WHEN** `veriplan check --json --compact` output is piped through `toon`
- **THEN** toon SHALL successfully parse and convert the output

### Requirement: Human output shows warning summary
`veriplan check` (default human mode) SHALL show warning and info summary counts after blockers, without expanding individual items.

#### Scenario: Default output has warning count
- **WHEN** `veriplan check` runs on a plan with warnings
- **THEN** the output SHALL contain a line like `N warning(s): ...` with the count

#### Scenario: Verbose output unchanged
- **WHEN** `veriplan check --verbose` runs
- **THEN** the output SHALL include full warning and info details (unchanged from current behavior)

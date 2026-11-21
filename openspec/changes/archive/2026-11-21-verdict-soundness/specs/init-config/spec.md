## ADDED Requirements

### Requirement: Init points the task reference table at the design artifact

T4.3 SHALL document the archive policy BEFORE T4.4 SHALL relocate the table rule.

The rule that `veriplan init` writes into `openspec/config.yaml` SHALL place the task reference table in the design artifact. The rule SHALL NOT instruct the author to add a section to a spec delta, because the archive does not carry an extra section into the published spec and reports a warning for every such section.

#### Scenario: The generated rule names the design artifact

- **GIVEN** a project without `openspec/config.yaml`
- **WHEN** T4.4 runs `veriplan init`
- **THEN** the generated `specs` rule SHALL name the design artifact as the place for the task reference table

#### Scenario: A delta carrying the table produces no warning

- **GIVEN** a change whose design artifact carries the task reference table
- **WHEN** `openspec validate` runs with `--strict`
- **THEN** the validation SHALL report no warning about a task reference section

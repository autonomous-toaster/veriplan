## Task Reference

| Task ID | Description |
|---------|-------------|
| T2.1 | Define PromelaModel, PromelaProcess, PromelaStmt types in src/ir/promela.rs |
| T2.2 | Refactor generate_promela() to return PromelaModel |
| T2.3 | Add promela_to_string() serialization function |
| T2.4 | Update Kani harnesses for Promela AST |

## ADDED Requirements

### Requirement: Promela AST types

The system SHALL define structured types for the Promela subset that veriplan generates, including variable declarations, process definitions, and LTL properties.

#### Scenario: PromelaModel contains all model elements

- **WHEN** a `PromelaModel` is constructed
- **THEN** it SHALL contain `variables`, `processes`, and `properties` collections

#### Scenario: PromelaStmt covers all statement types

- **WHEN** a Promela process body is constructed
- **THEN** it SHALL use `Assign`, `Do`, `If`, and `Break` variants as needed

### Requirement: Promela generation returns AST

`generate_promela()` SHALL return `PromelaModel` instead of `String`.

#### Scenario: Return type is PromelaModel

- **WHEN** `generate_promela()` is called with any valid PlanIR and constraints
- **THEN** it SHALL return a `PromelaModel` with one process per task

### Requirement: Promela serialization is lossless

`promela_to_string()` SHALL produce valid Promela source that SPIN can parse.

#### Scenario: Serialization produces valid Promela

- **WHEN** `promela_to_string()` serializes any `PromelaModel`
- **THEN** the output SHALL have balanced `{}`, balanced `do`/`od`, and valid variable declarations

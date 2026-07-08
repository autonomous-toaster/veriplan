## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Define LtlFormula and LtlCondition enums in src/ir/ltl.rs |
| T1.2 | Refactor generate_ltl() to return Option<LtlFormula> |
| T1.3 | Refactor evaluate_ltl() to take &LtlFormula |
| T1.4 | Add ltl_to_string() serialization function |
| T1.5 | Update Kani harnesses for LTL AST |
| T1.6 | Remove string-parsing logic from evaluate_ltl() |

## ADDED Requirements

### Requirement: LTL AST types

The system SHALL define `LtlFormula` and `LtlCondition` enums in `src/ir/ltl.rs` that represent all LTL patterns the translator generates.

#### Scenario: LtlFormula covers all temporal operators

- **WHEN** the translator generates an LTL formula for any constraint category
- **THEN** the formula SHALL be representable as an `LtlFormula::Always` or `LtlFormula::Eventually` variant

#### Scenario: LtlCondition covers all boolean connectives

- **WHEN** the translator generates an LTL condition
- **THEN** the condition SHALL be representable using `Atom`, `Not`, `And`, `Or`, `Implies`, or `Iff` variants

### Requirement: LTL generation returns AST

`generate_ltl()` SHALL return `Option<LtlFormula>` instead of `Option<String>`.

#### Scenario: Return type is LtlFormula

- **WHEN** `generate_ltl()` is called with any category and statement
- **THEN** it SHALL return `Some(LtlFormula)` for formalizable constraints and `None` for non-formalizable ones

### Requirement: LTL evaluation takes AST

`evaluate_ltl()` SHALL take `&LtlFormula` instead of `&str`.

#### Scenario: Evaluation uses structural induction

- **WHEN** `evaluate_ltl()` receives an `LtlFormula`
- **THEN** it SHALL evaluate it by matching on the enum variants, not by parsing strings

### Requirement: LTL serialization is lossless

`ltl_to_string()` SHALL produce the same string output as the current `format!()` calls for all LTL patterns.

#### Scenario: Serialization matches current output

- **WHEN** `ltl_to_string()` serializes any `LtlFormula`
- **THEN** the output SHALL match the format produced by the current `generate_ltl()` string formatting

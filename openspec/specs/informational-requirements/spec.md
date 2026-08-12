# Informational Requirements

## Purpose

Recognize and handle informational (human-review-only) requirements as non-blocking INFO items.

## Task Reference

| T ID | Description |
|------|-------------|
| T11.1 | classify a requirement as `Informational` |
| T11.2 | skip grounding and LTL generation for `Informational` requirements |
| T11.3 | report it |
## Requirements

### Requirement: Recognize informational requirements

T11.1 SHALL classify a requirement as `Informational` BEFORE T11.3 SHALL report it. A requirement SHALL be `Informational` when its statement contains a human-review-only marker such as "human review only", "informational", or "not formalizable by design", or when its RFC 2119 strength is `MAY`. An `Informational` requirement SHALL NOT be a temporal state-machine constraint.

#### Scenario: human-review-only marker classifies as Informational

- **GIVEN** a requirement whose body contains "human review only"
- **WHEN** veriplan classifies the requirement
- **THEN** the category SHALL be `Informational`
- **AND** it SHALL NOT match any temporal category

#### Scenario: MAY requirement is informational

- **GIVEN** a requirement with RFC 2119 strength `MAY`
- **WHEN** veriplan classifies the requirement
- **THEN** it SHALL be treated as informational

### Requirement: Informational requirements do not block

T11.2 SHALL skip grounding and LTL generation for `Informational` requirements AFTER T11.1 SHALL classify them. An `Informational` requirement SHALL NOT produce a non-formalizable blocker, SHALL NOT be grounded, and SHALL NOT generate LTL.

#### Scenario: informational requirement produces no blocker

- **GIVEN** a plan with one verifiable constraint and one `human review only` requirement
- **WHEN** veriplan checks convertibility in `Strict` mode
- **THEN** the plan SHALL be `Convertible`
- **AND** the informational requirement SHALL appear as an INFO item, not a blocker

#### Scenario: informational requirement is skipped in grounding

- **GIVEN** an informational requirement
- **WHEN** grounding runs
- **THEN** the requirement SHALL NOT be grounded
- **AND** no ungroundable/ambiguous finding SHALL be produced for it

### Requirement: Report informational requirements transparently

T11.3 SHALL surface `Informational` requirements as INFO check items AFTER T11.2 SHALL skip their analysis. The info detail SHALL state "human review only, not verified by model checking".

#### Scenario: informational requirement surfaced as info

- **GIVEN** an informational requirement
- **WHEN** veriplan produces the convertibility report
- **THEN** an INFO item SHALL appear with check `informational_requirement`
- **AND** its detail SHALL contain "human review only"

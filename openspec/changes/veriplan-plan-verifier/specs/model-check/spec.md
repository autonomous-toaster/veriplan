## ADDED Requirements

### Requirement: Generate Promela model from PlanIR

T6.1 SHALL generate the Promela model BEFORE T6.4 SHALL run SPIN. T6.1 AND T6.3 SHALL run CONCURRENTLY — T6.1 builds the model while T6.3 generates LTL properties.

#### Scenario: Single-phase Promela model
- **GIVEN** a PlanIR with 3 tasks (1.1, 1.2, 1.3) in one phase and one sequential constraint "T1.1 SHALL complete before T1.2"
- **WHEN** T6.1 runs
- **THEN** T6.1 SHALL produce a Promela file with:
  - `bool t1_1, t1_2, t1_3;` (state variables)
  - `active proctype task_t1_1()` with `do :: (1) -> t1_1 = done; break od`
  - `ltl p0 { G (active(t1_2) -> done(t1_1)) }` (sequential constraint)

#### Scenario: Multi-phase model with ordering
- **GIVEN** a PlanIR with Phase 1 (tasks 1.1-1.3) and Phase 2 (tasks 2.1-2.3) where T2.1 SHALL complete AFTER all Phase 1 tasks
- **WHEN** T6.1 runs
- **THEN** T6.2 SHALL add a guard: Phase 2 tasks can only activate AFTER all of t1_1, t1_2, t1_3 are done

#### Scenario: Exclusive constraint in Promela
- **GIVEN** exclusive constraint "At most one of T2.1 and T2.2 SHALL be active"
- **WHEN** T6.1 runs
- **THEN** T6.3 SHALL include: `ltl p1 { G ( !(t2_1 == 1 && t2_2 == 1) ) }`

### Requirement: Run SPIN model checker

T6.4 SHALL run SPIN AFTER T6.1 SHALL generate the Promela model. SPIN SHALL be discovered on PATH — if missing, the checker SHALL fail hard.

#### Scenario: SPIN available, model valid
- **GIVEN** the `spin` binary is on PATH and the Promela model is consistent
- **WHEN** T6.4 runs `spin -run -ltp model.pml`
- **THEN** T6.4 SHALL return success for all LTL properties
- **AND** the plan SHALL be marked VALID

#### Scenario: SPIN available, ordering violation
- **GIVEN** a Promela model where task 1.2 can activate before task 1.1
- **WHEN** T6.4 runs
- **THEN** SPIN SHALL detect the violation
- **AND** T6.6 SHALL parse the counterexample trail
- **AND** T7.1 SHALL mark the plan INVALID

#### Scenario: SPIN available, deadlock detected
- **GIVEN** a Promela model where tasks form a circular dependency (1.1→1.2→1.1)
- **WHEN** T6.4 runs
- **THEN** SPIN SHALL report invalid end state (deadlock)
- **AND** T7.1 SHALL mark the plan INVALID

### Requirement: Built-in BFS fallback

T6.7 SHALL provide a built-in BFS explorer IF SPIN is not available AND the plan has ≤20 tasks. (Deprecated: T6.4 now SHALL fail hard when SPIN is missing.)

#### Scenario: SPIN not available, model valid
- **GIVEN** `spin` binary is not installed and a plan with 4 tasks and 2 LTL properties
- **WHEN** T6.4 fails to find SPIN
- **THEN** T6.4 SHALL emit a hard error: "SPIN binary not found on PATH"
- **AND** the plan SHALL be marked NOT CONVERTIBLE

### Requirement: Annotate counterexample with source locations

T7.1 SHALL map each counterexample to source locations AFTER T6.6 SHALL parse the trail file. T7.1 AND T7.2 SHALL run CONCURRENTLY.

#### Scenario: Ordering violation annotated
- **GIVEN** a SPIN trail showing task 1.2 active before task 1.1 done
- **WHEN** T7.1 runs
- **THEN** T7.1 SHALL produce:
  - Violated requirement: `specs/deploy/spec.md:16` (sequential constraint)
  - Violating task: `tasks.md:13` (task 1.2)
  - State: `t1_1=0, t1_2=1`

#### Scenario: Multiple violations
- **GIVEN** a plan with a sequential violation AND an exclusive constraint violation
- **WHEN** T7.1 runs
- **THEN** T7.1 SHALL list both violations with source locations for each

### Requirement: Generate human-readable and JSON report

T7.3 SHALL generate the final report AFTER T7.1 SHALL complete annotation. T7.3 AND T7.4 SHALL run CONCURRENTLY for human and JSON formats.

#### Scenario: VALID report
- **GIVEN** a plan where all LTL properties pass
- **WHEN** T7.3 runs
- **THEN** stdout SHALL show:
  ```
  ✓ Plan: VALID
    - All 5 properties satisfied
  ```
- **AND** T7.4 SHALL produce JSON with `{"plan": "veriplan-plan-verifier", "valid": true, "violations": []}`

#### Scenario: INVALID report with violations
- **GIVEN** a plan with 2 ordering violations and 1 exclusive violation
- **WHEN** T7.3 runs
- **THEN** stdout SHALL show each violation with source locations and suggested fix
- **AND** T7.4 SHALL produce JSON with all violations structured

#### Scenario: Blocking convertibility
- **GIVEN** a plan that failed convertibility check
- **WHEN** T4.10 runs
- **THEN** stdout SHALL show:
  ```
  ✗ Plan: NOT CONVERTIBLE (3 blockers)
  - [BLOCKER] Requirement references non-existent task T99 (specs/deploy/spec.md:24)
  - [BLOCKER] No classifiable requirements (specs/quality/spec.md:12)
  - [WARNING] Scenario 'Rollback' missing THEN (specs/deploy/spec.md:45)

  Rephrase the spec and re-run.
  ```

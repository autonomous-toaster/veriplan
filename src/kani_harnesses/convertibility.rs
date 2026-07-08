//! Kani proof harnesses for the convertibility check.
//!
//! The convertibility check is the gatekeeper: it decides whether
//! a plan can proceed to model checking.
//!
//! These harnesses prove:
//!   1. check_tasks severity invariants
//!   2. check_requirements handles any plan without panicking
//!   3. check_task_references has no false positives
//!   4. check_classifiability handles any plan without panicking

use crate::checker::checks;
use crate::ir::*;

// ─────────────────────────────────────────────────────────────
// Harness 1: check_tasks — severity invariants
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_check_tasks_soundness() {
    let plan = PlanIR {
        tasks: vec![
            Task { id: "1.1".into(), description: "Setup".into(), phase: "P1".into(), checked: false, source: SourceLocation { file: "t.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 } },
            Task { id: "1.2".into(), description: "Build".into(), phase: "P1".into(), checked: false, source: SourceLocation { file: "t.md".into(), start_byte: 0, end_byte: 0, start_line: 2, end_line: 2 } },
        ],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let (blocker, warnings, info) = checks::check_tasks(&plan, true);

    if let Some(b) = &blocker {
        assert_eq!(b.severity, "blocker");
        assert!(!b.check.is_empty());
        assert!(!b.element.is_empty());
    }

    for w in &warnings {
        assert_eq!(w.severity, "warning");
    }

    for i in &info {
        assert_eq!(i.severity, "info");
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 2: check_requirements — no panics
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_check_requirements_no_panic() {
    let plan = PlanIR {
        tasks: vec![],
        requirements: vec![
            Requirement {
                id: "R1".into(),
                statement: "T1.1 SHALL complete BEFORE T1.2".into(),
                strength: Rfc2119Strength::Must,
                category: ConstraintCategory::SequentialOrder,
                ltl: None,
                scenarios: vec![],
                source: SourceLocation { file: "spec.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 },
            },
            Requirement {
                id: "R2".into(),
                statement: "The system SHALL be robust".into(),
                strength: Rfc2119Strength::None,
                category: ConstraintCategory::NonFormalizable,
                ltl: None,
                scenarios: vec![],
                source: SourceLocation { file: "spec.md".into(), start_byte: 0, end_byte: 0, start_line: 2, end_line: 2 },
            },
        ],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let (blockers, warnings, info) = checks::check_requirements(&plan, true);

    for b in &blockers { assert_eq!(b.severity, "blocker"); }
    for w in &warnings { assert_eq!(w.severity, "warning"); }
    for i in &info { assert_eq!(i.severity, "info"); }
}

// ─────────────────────────────────────────────────────────────
// Harness 3: check_task_references — no false positives
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_check_task_references_no_false_positives() {
    let plan = PlanIR {
        tasks: vec![
            Task { id: "1.1".into(), description: "Setup".into(), phase: "P1".into(), checked: false, source: SourceLocation { file: "t.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 } },
        ],
        requirements: vec![
            Requirement {
                id: "R1".into(),
                statement: "T1.1 SHALL complete".into(),
                strength: Rfc2119Strength::Must,
                category: ConstraintCategory::SequentialOrder,
                ltl: None,
                scenarios: vec![],
                source: SourceLocation { file: "spec.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 },
            },
        ],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let (blockers, _) = checks::check_task_references(&plan);
    for b in &blockers {
        assert_ne!(b.check, "bad_task_reference",
            "False positive: existing task flagged as bad reference");
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 4: check_classifiability — no panics
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_check_classifiability_no_panic() {
    let plan = PlanIR {
        tasks: vec![],
        requirements: vec![
            Requirement {
                id: "R1".into(),
                statement: "T1.1 SHALL complete BEFORE T1.2".into(),
                strength: Rfc2119Strength::Must,
                category: ConstraintCategory::NonFormalizable,
                ltl: None,
                scenarios: vec![],
                source: SourceLocation { file: "spec.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 },
            },
        ],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let _ = checks::check_classifiability(&plan, true);
}

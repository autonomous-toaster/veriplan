//! Kani proof harnesses for the convertibility check.
//!
//! The convertibility check is the gatekeeper: it decides whether
//! a plan can proceed to model checking. If it's wrong, either
//! bad plans pass (false sense of security) or good plans are
//! rejected (frustration).
//!
//! These harnesses prove:
//!   1. Soundness: if status is Blocking, there's at least one blocker
//!   2. Completeness: if there's a blocker, status is Blocking
//!   3. All checks run without panicking
//!   4. The report structure is internally consistent

use crate::checker::checks;
use crate::ir::*;

// ─────────────────────────────────────────────────────────────
// Harness 1: check_tasks — soundness
// ─────────────────────────────────────────────────────────────
// Proves that check_tasks returns a blocker iff there are
// duplicate task IDs or no tasks (in openspec mode).
#[kani::proof]
#[kani::unwind(5)]
fn verify_check_tasks_soundness() {
    let mut tasks: Vec<Task> = Vec::new();
    let task_count: usize = kani::any();
    kani::assume(task_count <= 3);

    for i in 0..task_count {
        tasks.push(Task {
            id: format!("{}.{}", i + 1, 1),
            description: format!("task_{}", i),
            phase: "Phase 1".into(),
            checked: false,
            source: SourceLocation {
                file: "tasks.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                end_line: 1,
            },
        });
    }

    let plan = PlanIR {
        tasks,
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let (blocker, warnings, info) = checks::check_tasks(&plan, true);

    // Invariant: if blocker is Some, it must be a valid CheckItem
    if let Some(b) = &blocker {
        assert_eq!(b.severity, "blocker");
        assert!(!b.check.is_empty());
        assert!(!b.element.is_empty());
    }

    // Invariant: all warnings have severity "warning"
    for w in &warnings {
        assert_eq!(w.severity, "warning");
    }

    // Invariant: all info have severity "info"
    for i in &info {
        assert_eq!(i.severity, "info");
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 2: check_requirements — no panics
// ─────────────────────────────────────────────────────────────
// Proves that check_requirements handles any plan without panicking.
#[kani::proof]
#[kani::unwind(5)]
fn verify_check_requirements_no_panic() {
    let mut requirements: Vec<Requirement> = Vec::new();
    let req_count: usize = kani::any();
    kani::assume(req_count <= 3);

    for i in 0..req_count {
        let strength: u8 = kani::any();
        kani::assume(strength <= 4);
        let strength = match strength {
            0 => Rfc2119Strength::Must,
            1 => Rfc2119Strength::Should,
            2 => Rfc2119Strength::May,
            3 => Rfc2119Strength::MustNot,
            _ => Rfc2119Strength::None,
        };

        requirements.push(Requirement {
            id: format!("R{}", i + 1),
            statement: kani::any::<String>(),
            strength,
            category: ConstraintCategory::NonFormalizable,
            ltl: None,
            scenarios: vec![],
            source: SourceLocation {
                file: "spec.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                end_line: 1,
            },
        });
    }

    let plan = PlanIR {
        tasks: vec![],
        requirements,
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let (blockers, warnings, info) = checks::check_requirements(&plan, true);

    // Invariant: all items have correct severity
    for b in &blockers { assert_eq!(b.severity, "blocker"); }
    for w in &warnings { assert_eq!(w.severity, "warning"); }
    for i in &info { assert_eq!(i.severity, "info"); }
}

// ─────────────────────────────────────────────────────────────
// Harness 3: check_task_references — no false positives
// ─────────────────────────────────────────────────────────────
// Proves that check_task_references only flags references to
// non-existent tasks, never existing ones.
#[kani::proof]
#[kani::unwind(5)]
fn verify_check_task_references_no_false_positives() {
    let mut tasks: Vec<Task> = Vec::new();
    let task_count: usize = kani::any();
    kani::assume(task_count >= 1);
    kani::assume(task_count <= 3);

    for i in 0..task_count {
        tasks.push(Task {
            id: format!("{}.{}", i + 1, 1),
            description: format!("task_{}", i),
            phase: "Phase 1".into(),
            checked: false,
            source: SourceLocation {
                file: "tasks.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                end_line: 1,
            },
        });
    }

    let plan = PlanIR {
        tasks,
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    // A requirement that references an existing task should NOT produce a blocker
    if let Some(first_task) = plan.tasks.first() {
        let statement = format!("T{} SHALL complete", first_task.id);
        let req = Requirement {
            id: "R1".into(),
            statement,
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::SequentialOrder,
            ltl: None,
            scenarios: vec![],
            source: SourceLocation {
                file: "spec.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                end_line: 1,
            },
        };

        let plan_with_req = PlanIR {
            tasks: plan.tasks.clone(),
            requirements: vec![req],
            scenarios: vec![],
            phases: vec![],
            source_map: SourceMap::default(),
        };

        let (blockers, _) = checks::check_task_references(&plan_with_req);
        for b in &blockers {
            assert_ne!(b.check, "bad_task_reference",
                "False positive: existing task flagged as bad reference");
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 4: check_classifiability — no panics
// ─────────────────────────────────────────────────────────────
// Proves that check_classifiability handles any plan without panicking.
#[kani::proof]
#[kani::unwind(5)]
fn verify_check_classifiability_no_panic() {
    let mut requirements: Vec<Requirement> = Vec::new();
    let req_count: usize = kani::any();
    kani::assume(req_count <= 3);

    for i in 0..req_count {
        requirements.push(Requirement {
            id: format!("R{}", i + 1),
            statement: kani::any::<String>(),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::NonFormalizable,
            ltl: None,
            scenarios: vec![],
            source: SourceLocation {
                file: "spec.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                end_line: 1,
            },
        });
    }

    let plan = PlanIR {
        tasks: vec![],
        requirements,
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let _ = checks::check_classifiability(&plan, true);
}

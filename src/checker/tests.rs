//! Tests for the checker result types, kept out of mod.rs to stay under the file-length gate.
// Tests fail by panicking; the crate denies these three lints for code that ships.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::ir::CheckItem;

fn make_result(
    plan_name: &str,
    convertible: bool,
    valid: Option<bool>,
    violations: usize,
) -> VerificationResult {
    VerificationResult {
        plan_name: plan_name.into(),
        phase: "full".into(),
        convertible,
        convertibility_report: None,
        valid,
        violations: (0..violations)
            .map(|i| Violation {
                constraint_id: format!("R{}", i),
                requirement_statement: "test".into(),
                ltl: String::new(),
                category: String::new(),
                state: String::new(),
                task_source: None,
                req_source: None,
                suggested_fix: None,
                plan: plan_name.into(),
                kind: crate::ir::Kind::ProseOther,
                op: crate::ir::Op::None,
            })
            .collect(),
        total_constraints: 10,
        satisfied_constraints: 5,
        unmodelled_constraints: 0,
        unchecked_constraints: 0,
        historical_constraints: 0,
        backend: "test".into(),
        constraints_summary: vec![],
        skip_reason: None,
    }
}

#[test]
fn test_drain_by_severity() {
    let mut items = vec![
        CheckItem {
            check: "test".into(),
            element: "e1".into(),
            location: "l1".into(),
            detail: "d1".into(),
            severity: "blocker".into(),
            fix: None,
            kind: crate::ir::Kind::ProseOther,
            op: crate::ir::Op::None,
            fixability: crate::ir::Fixability::Structural,
            start: 0,
            end: 0,
            replacement: None,
        },
        CheckItem {
            check: "test".into(),
            element: "e2".into(),
            location: "l2".into(),
            detail: "d2".into(),
            severity: "warning".into(),
            fix: None,
            kind: crate::ir::Kind::ProseOther,
            op: crate::ir::Op::None,
            fixability: crate::ir::Fixability::Structural,
            start: 0,
            end: 0,
            replacement: None,
        },
        CheckItem {
            check: "test".into(),
            element: "e3".into(),
            location: "l3".into(),
            detail: "d3".into(),
            severity: "info".into(),
            fix: None,
            kind: crate::ir::Kind::ProseOther,
            op: crate::ir::Op::None,
            fixability: crate::ir::Fixability::Structural,
            start: 0,
            end: 0,
            replacement: None,
        },
    ];
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();
    drain_by_severity(&mut items, &mut blockers, &mut warnings, &mut info);
    assert_eq!(blockers.len(), 1);
    assert_eq!(warnings.len(), 1);
    assert_eq!(info.len(), 1);
    assert!(items.is_empty());
}

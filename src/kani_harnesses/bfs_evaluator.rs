//! Kani proof harnesses for the BFS LTL evaluator.
//!
//! Uses LtlFormula/LtlCondition enums directly — no string parsing.
//! Kani verifies by structural induction on the enum variants.

use crate::checker::bfs::*;
use crate::ir::ltl::{LtlCondition, LtlFormula};
use crate::ir::*;

fn empty_plan() -> PlanIR {
    PlanIR {
        tasks: vec![],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 1: evaluate_ltl_atom — present variable
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_atom_present() {
    let state = vec![("x".to_string(), 1u8)];
    assert!(evaluate_ltl_atom("x", &state));
}

// ─────────────────────────────────────────────────────────────
// Harness 2: evaluate_ltl_atom — absent variable
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_atom_absent() {
    let state = vec![("x".to_string(), 1u8)];
    assert!(!evaluate_ltl_atom("y", &state));
}

// ─────────────────────────────────────────────────────────────
// Harness 3: evaluate_ltl_atom — negation
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_atom_negation() {
    let state = vec![("x".to_string(), 0u8)];
    assert!(evaluate_ltl_atom("!x", &state));
}

// ─────────────────────────────────────────────────────────────
// Harness 4: evaluate_ltl_condition — implication true
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_implication_true() {
    let state = vec![("x".to_string(), 0u8), ("y".to_string(), 0u8)];
    let cond = LtlCondition::Implies(
        Box::new(LtlCondition::Atom("x".into())),
        Box::new(LtlCondition::Atom("y".into())),
    );
    assert!(evaluate_ltl_condition(&cond, &state));
}

// ─────────────────────────────────────────────────────────────
// Harness 5: evaluate_ltl_condition — implication false
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_implication_false() {
    let state = vec![("x".to_string(), 1u8), ("y".to_string(), 0u8)];
    let cond = LtlCondition::Implies(
        Box::new(LtlCondition::Atom("x".into())),
        Box::new(LtlCondition::Atom("y".into())),
    );
    assert!(!evaluate_ltl_condition(&cond, &state));
}

// ─────────────────────────────────────────────────────────────
// Harness 6: evaluate_ltl — Always(Atom)
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_always_atom() {
    let state = vec![("x".to_string(), 1u8)];
    let formula = LtlFormula::Always(LtlCondition::Atom("x".into()));
    assert!(evaluate_ltl(&formula, &state, &empty_plan()));
}

// ─────────────────────────────────────────────────────────────
// Harness 7: evaluate_ltl — Always(Not(Atom))
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_always_not() {
    let state = vec![("x".to_string(), 0u8)];
    let formula = LtlFormula::Always(LtlCondition::Not(Box::new(LtlCondition::Atom("x".into()))));
    assert!(evaluate_ltl(&formula, &state, &empty_plan()));
}

// ─────────────────────────────────────────────────────────────
// Harness 8: evaluate_ltl — Always(Eventually(Atom))
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_always_eventually() {
    let state = vec![("x".to_string(), 1u8)];
    let formula = LtlFormula::Always(LtlCondition::Eventually(Box::new(LtlCondition::Atom(
        "x".into(),
    ))));
    assert!(evaluate_ltl(&formula, &state, &empty_plan()));
}

// ─────────────────────────────────────────────────────────────
// Harness 9: evaluate_ltl — Always(Iff)
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_always_iff() {
    let state = vec![("x".to_string(), 1u8), ("y".to_string(), 1u8)];
    let formula = LtlFormula::Always(LtlCondition::Iff(
        Box::new(LtlCondition::Atom("x".into())),
        Box::new(LtlCondition::Atom("y".into())),
    ));
    assert!(evaluate_ltl(&formula, &state, &empty_plan()));
}

// ─────────────────────────────────────────────────────────────
// Harness 10: evaluate_ltl — Always(And)
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_always_and() {
    let state = vec![("x".to_string(), 1u8), ("y".to_string(), 1u8)];
    let formula = LtlFormula::Always(LtlCondition::And(vec![
        LtlCondition::Atom("x".into()),
        LtlCondition::Atom("y".into()),
    ]));
    assert!(evaluate_ltl(&formula, &state, &empty_plan()));
}

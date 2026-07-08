//! Kani proof harnesses for the LTL translator.
//!
//! Tests classification with minimal string operations.

use crate::ir::*;
use crate::translator;

// ─────────────────────────────────────────────────────────────
// Harness 1: classify — sequential (BEFORE)
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_classify_sequential() {
    let result = translator::classify("T1.1 SHALL complete BEFORE T1.2");
    assert_eq!(result, ConstraintCategory::SequentialOrder);
}

// ─────────────────────────────────────────────────────────────
// Harness 2: classify — exclusive (AT MOST ONE)
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_classify_exclusive() {
    let result = translator::classify("At most one of T1.1, T1.2 SHALL be active");
    assert_eq!(result, ConstraintCategory::Exclusive);
}

// ─────────────────────────────────────────────────────────────
// Harness 3: classify — conditional (IF...THEN)
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_classify_conditional() {
    let result = translator::classify("IF T1.1 fails THEN T2.1 SHALL run");
    assert_eq!(result, ConstraintCategory::Conditional);
}

// ─────────────────────────────────────────────────────────────
// Harness 4: classify — concurrent
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_classify_concurrent() {
    let result = translator::classify("T3.1 and T3.2 SHALL run concurrently");
    assert_eq!(result, ConstraintCategory::ConcurrentEvents);
}

// ─────────────────────────────────────────────────────────────
// Harness 5: classify — non-formalizable
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_classify_non_formalizable() {
    let result = translator::classify("The system SHALL be robust");
    assert_eq!(result, ConstraintCategory::NonFormalizable);
}

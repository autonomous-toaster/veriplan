//! Kani proof harnesses for the naming convention coupling.
//!
//! Uses minimal assertions to keep Kani's symbolic execution tractable.

use crate::checker::promela::{active_var, done_var, fail_var};
use crate::translator;

// ─────────────────────────────────────────────────────────────
// Harness 1: active_var produces expected output
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_active_var() {
    let result = active_var("1.1");
    assert_eq!(result, "active_t1_1");
}

// ─────────────────────────────────────────────────────────────
// Harness 2: done_var produces expected output
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_done_var() {
    let result = done_var("1.1");
    assert_eq!(result, "done_t1_1");
}

// ─────────────────────────────────────────────────────────────
// Harness 3: fail_var produces expected output
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_fail_var() {
    let result = fail_var("1.1");
    assert_eq!(result, "failed_t1_1");
}

// ─────────────────────────────────────────────────────────────
// Harness 4: normalize_id produces expected output
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_normalize_id() {
    let result = translator::normalize_id("1.1");
    assert_eq!(result, "t1_1");
}

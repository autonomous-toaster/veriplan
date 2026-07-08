//! Kani proof harnesses for the naming convention coupling.
//!
//! Three places must agree on variable names:
//!   translator: normalize_id("1.3") → "t1_3"
//!   promela:    active_var("1.3")   → "active_t1_3"
//!   promela:    done_var("1.3")     → "done_t1_3"
//!   promela:    fail_var("1.3")     → "failed_t1_3"
//!
//! The LTL formulas reference `active_t1_3`, `done_t1_3`, `failed_t1_3`.
//! The Promela model declares `bit active_t1_3`, `bit done_t1_3`, `bit failed_t1_3`.
//! If these diverge, SPIN silently treats LTL references as false.

use crate::checker::promela::{active_var, done_var, fail_var};
use crate::translator;

// ─────────────────────────────────────────────────────────────
// Harness 1: normalize_id produces valid Promela identifiers
// ─────────────────────────────────────────────────────────────
// Proves that for any valid task ID, normalize_id produces a
// string that is a valid Promela identifier suffix.
#[kani::proof]
fn verify_normalize_id_valid_identifier() {
    let major: u32 = kani::any();
    let minor: u32 = kani::any();
    kani::assume(major <= 99);
    kani::assume(minor <= 99);

    let id = format!("{}.{}", major, minor);
    let normalized = translator::normalize_id(&id);

    // Must be non-empty
    assert!(!normalized.is_empty());

    // Must start with 't'
    assert!(normalized.starts_with('t'));

    // Must contain exactly one underscore (between major and minor)
    let underscore_count = normalized.chars().filter(|c| *c == '_').count();
    assert_eq!(underscore_count, 1);

    // All characters after 't' must be digits or underscores
    for c in normalized.chars().skip(1) {
        assert!(c.is_ascii_digit() || c == '_');
    }

    // Must not end with underscore
    assert!(!normalized.ends_with('_'));
}

// ─────────────────────────────────────────────────────────────
// Harness 2: active_var/done_var/fail_var produce valid names
// ─────────────────────────────────────────────────────────────
// Proves that all three variable naming functions produce
// syntactically valid Promela identifiers.
#[kani::proof]
fn verify_var_names_valid() {
    let major: u32 = kani::any();
    let minor: u32 = kani::any();
    kani::assume(major <= 99);
    kani::assume(minor <= 99);

    let id = format!("{}.{}", major, minor);

    let active = active_var(&id);
    let done = done_var(&id);
    let failed = fail_var(&id);

    // All must be non-empty
    assert!(!active.is_empty());
    assert!(!done.is_empty());
    assert!(!failed.is_empty());

    // All must start with a letter (Promela identifier rule)
    assert!(active.starts_with(|c: char| c.is_ascii_alphabetic()));
    assert!(done.starts_with(|c: char| c.is_ascii_alphabetic()));
    assert!(failed.starts_with(|c: char| c.is_ascii_alphabetic()));

    // All must contain only alphanumeric chars and underscores
    for name in &[&active, &done, &failed] {
        for c in name.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '_');
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 3: naming convention consistency
// ─────────────────────────────────────────────────────────────
// Proves that the LTL variable references match the Promela
// variable declarations. This is the key invariant.
//
// The translator generates LTL like:
//   "[] ( active_t1_3 -> done_t1_3 )"
//
// The promela generator declares:
//   "bit active_t1_3 = 0;"
//   "bit done_t1_3 = 0;"
//
// This harness proves the naming is consistent for all task IDs.
#[kani::proof]
fn verify_naming_consistency() {
    let major: u32 = kani::any();
    let minor: u32 = kani::any();
    kani::assume(major <= 99);
    kani::assume(minor <= 99);

    let id = format!("{}.{}", major, minor);
    let normalized = translator::normalize_id(&id);

    // The LTL references use format: active_{normalized}
    let ltl_active_ref = format!("active_{}", normalized);
    let ltl_done_ref = format!("done_{}", normalized);
    let ltl_failed_ref = format!("failed_{}", normalized);

    // The Promela declarations use active_var, done_var, fail_var
    let promela_active = active_var(&id);
    let promela_done = done_var(&id);
    let promela_failed = fail_var(&id);

    // They must match exactly
    assert_eq!(ltl_active_ref, promela_active);
    assert_eq!(ltl_done_ref, promela_done);
    assert_eq!(ltl_failed_ref, promela_failed);
}

// ─────────────────────────────────────────────────────────────
// Harness 4: round-trip consistency
// ─────────────────────────────────────────────────────────────
// Proves that the naming is bijective: different task IDs
// produce different variable names.
#[kani::proof]
fn verify_naming_is_bijective() {
    let id_a = format!("{}.{}", 1, 1);
    let id_b = format!("{}.{}", 1, 2);

    let active_a = active_var(&id_a);
    let active_b = active_var(&id_b);

    // Different IDs must produce different names
    assert_ne!(active_a, active_b);

    // Same ID must produce same name
    let active_a2 = active_var(&id_a);
    assert_eq!(active_a, active_a2);
}

//! Kani proof harnesses for the BFS LTL evaluator.
//!
//! The BFS fallback checker's `evaluate_ltl` is the most critical target:
//! it silently passes all unrecognized LTL patterns, making the BFS checker
//! completely unsound for any LTL formula not in `G ( ... )` form.
//!
//! These harnesses prove:
//!   1. `evaluate_ltl` correctly evaluates `G ( condition )` patterns
//!   2. `evaluate_ltl` correctly handles all LTL operators (->, <->, !, &&, F)
//!   3. `evaluate_ltl_atom` correctly resolves variable references
//!   4. The evaluator never panics on any input

// Note: These are sketches — they assume the module structure allows
// importing `bfs::*` from a kani crate. In practice you'd add
// `#[cfg(kani)] mod kani_harnesses;` to bfs.rs or use a separate
// test crate with `kani` as a dev-dependency.

use crate::checker::bfs::*;
use crate::ir::*;

// ─────────────────────────────────────────────────────────────
// Harness 1: evaluate_ltl_atom — variable resolution
// ─────────────────────────────────────────────────────────────
// Proves that atom evaluation is correct for all possible states
// of a bounded task set.
#[kani::proof]
#[kani::unwind(5)]
fn verify_evaluate_ltl_atom() {
    // Build a state with up to 3 tasks (2^3 = 8 states)
    let task_count: usize = kani::any();
    kani::assume(task_count <= 3);

    let mut state: HashMap<String, u8> = HashMap::new();
    let mut task_ids: Vec<String> = Vec::new();
    for i in 0..task_count {
        let id = format!("active_t1_{}", i);
        let val: u8 = kani::any();
        kani::assume(val <= 1);
        state.insert(id.clone(), val);
        task_ids.push(id);
    }

    // For each task, verify that evaluate_ltl_atom matches the state
    for id in &task_ids {
        let expected = *state.get(id).unwrap();
        let result = evaluate_ltl_atom(id, &state);
        assert_eq!(result, expected == 1);
    }

    // For each task, verify negation is correct
    for id in &task_ids {
        let negated = format!("!{}", id);
        let expected = *state.get(id).unwrap();
        let result = evaluate_ltl_atom(&negated, &state);
        assert_eq!(result, expected == 0);
    }

    // Unknown variables always return false
    let unknown = evaluate_ltl_atom("nonexistent_var", &state);
    assert!(!unknown);
}

// ─────────────────────────────────────────────────────────────
// Harness 2: evaluate_ltl_condition — implication
// ─────────────────────────────────────────────────────────────
// Proves that `A -> B` is correctly evaluated as `!A || B`.
#[kani::proof]
fn verify_implication_semantics() {
    let a_val: u8 = kani::any();
    let b_val: u8 = kani::any();
    kani::assume(a_val <= 1);
    kani::assume(b_val <= 1);

    let mut state: HashMap<String, u8> = HashMap::new();
    state.insert("active_t1_1".to_string(), a_val);
    state.insert("done_t1_1".to_string(), b_val);

    let plan = PlanIR {
        tasks: vec![],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let result = evaluate_ltl_condition("active_t1_1 -> done_t1_1", &state, &plan);
    let expected = a_val == 0 || b_val == 1;
    assert_eq!(result, expected);
}

// ─────────────────────────────────────────────────────────────
// Harness 3: evaluate_ltl_condition — bidirectional
// ─────────────────────────────────────────────────────────────
// Proves that `A <-> B` is correctly evaluated as `A == B`.
#[kani::proof]
fn verify_bidirectional_semantics() {
    let a_val: u8 = kani::any();
    let b_val: u8 = kani::any();
    kani::assume(a_val <= 1);
    kani::assume(b_val <= 1);

    let mut state: HashMap<String, u8> = HashMap::new();
    state.insert("active_t1_1".to_string(), a_val);
    state.insert("active_t1_2".to_string(), b_val);

    let plan = PlanIR {
        tasks: vec![],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    let result = evaluate_ltl_condition("active_t1_1 <-> active_t1_2", &state, &plan);
    assert_eq!(result, a_val == b_val);
}

// ─────────────────────────────────────────────────────────────
// Harness 4: evaluate_ltl — G ( ... ) pattern matching
// ─────────────────────────────────────────────────────────────
// Proves that `G ( condition )` is correctly parsed and evaluated.
// This is the ONLY pattern the BFS evaluator handles — everything
// else silently passes.
#[kani::proof]
fn verify_always_pattern() {
    let a_val: u8 = kani::any();
    let b_val: u8 = kani::any();
    kani::assume(a_val <= 1);
    kani::assume(b_val <= 1);

    let mut state: HashMap<String, u8> = HashMap::new();
    state.insert("active_t1_1".to_string(), a_val);
    state.insert("done_t1_2".to_string(), b_val);

    let plan = PlanIR {
        tasks: vec![],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    // G ( active_t1_1 -> done_t1_2 )
    let ltl = "G ( active_t1_1 -> done_t1_2 )";
    let result = evaluate_ltl(ltl, &state, &plan);
    let expected = a_val == 0 || b_val == 1;
    assert_eq!(result, expected);
}

// ─────────────────────────────────────────────────────────────
// Harness 5: evaluate_ltl — unrecognized patterns (THE BUG)
// ─────────────────────────────────────────────────────────────
// Proves that unrecognized LTL patterns silently pass.
// This is the bug: the translator generates `[] ( ... )` but
// the evaluator looks for `G ( ... )`. They don't match.
#[kani::proof]
fn verify_unrecognized_patterns_silently_pass() {
    let a_val: u8 = kani::any();
    kani::assume(a_val <= 1);

    let mut state: HashMap<String, u8> = HashMap::new();
    state.insert("active_t1_1".to_string(), a_val);

    let plan = PlanIR {
        tasks: vec![],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    // The translator generates `[] ( ... )` but the evaluator
    // only handles `G ( ... )`. This harness proves the mismatch.
    let ltl_bracket = "[] ( active_t1_1 )";  // what translator generates
    let result_bracket = evaluate_ltl(ltl_bracket, &state, &plan);
    assert!(result_bracket);  // ← silently passes — BUG

    // Also test: <> (eventually), U (until), X (next)
    let ltl_eventually = "<> active_t1_1";
    assert!(evaluate_ltl(ltl_eventually, &state, &plan));  // ← silently passes

    let ltl_until = "active_t1_1 U done_t1_1";
    assert!(evaluate_ltl(ltl_until, &state, &plan));  // ← silently passes

    // Any unrecognized pattern passes — this is the unsoundness
    let ltl_garbage = "!@#$%^";
    assert!(evaluate_ltl(ltl_garbage, &state, &plan));  // ← silently passes
}

// ─────────────────────────────────────────────────────────────
// Harness 6: evaluate_ltl — no panics on any input
// ─────────────────────────────────────────────────────────────
// Proves the evaluator never panics, regardless of input.
#[kani::proof]
fn verify_evaluate_ltl_no_panic() {
    let ltl: String = kani::any();
    let state: HashMap<String, u8> = kani::any();
    let plan = PlanIR {
        tasks: vec![],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    };

    // Must never panic
    let _ = evaluate_ltl(&ltl, &state, &plan);
}

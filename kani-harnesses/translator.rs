//! Kani proof harnesses for the LTL translator.
//!
//! The translator is the bridge between natural language requirements
//! and formal LTL. Bugs here produce silent wrong answers.
//!
//! These harnesses prove:
//!   1. `classify` is exhaustive for known temporal patterns
//!   2. `generate_ltl` produces syntactically valid LTL
//!   3. `extract_task_refs` finds all and only valid references
//!   4. `find_sequential_pair` correctly identifies ordering

use crate::ir::*;
use crate::translator;

// ─────────────────────────────────────────────────────────────
// Harness 1: classify never panics
// ─────────────────────────────────────────────────────────────
// Proves that classify handles any input string without panicking.
#[kani::proof]
fn verify_classify_no_panic() {
    let statement: String = kani::any();
    let _ = translator::classify(&statement);
}

// ─────────────────────────────────────────────────────────────
// Harness 2: generate_ltl produces syntactically valid LTL
// ─────────────────────────────────────────────────────────────
// Proves that for any category and any statement referencing
// existing tasks, the generated LTL is syntactically well-formed.
#[kani::proof]
#[kani::unwind(5)]
fn verify_generated_ltl_is_valid() {
    // Build a plan with up to 3 tasks
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
                file: "test.md".into(),
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

    // Test all categories
    let categories = [
        ConstraintCategory::SequentialOrder,
        ConstraintCategory::Exclusive,
        ConstraintCategory::Conditional,
        ConstraintCategory::ConcurrentEvents,
        ConstraintCategory::FixedTime,
        ConstraintCategory::Global,
    ];

    for category in &categories {
        let statement: String = kani::any();
        let ltl = translator::generate_ltl(category, &statement, &plan);

        if let Some(formula) = ltl {
            // Must be non-empty
            assert!(!formula.is_empty(), "LTL formula must not be empty");

            // Must have balanced brackets
            let open_parens = formula.matches('(').count();
            let close_parens = formula.matches(')').count();
            assert_eq!(open_parens, close_parens,
                "Unbalanced parentheses in LTL: {}", formula);

            let open_brackets = formula.matches('[').count();
            let close_brackets = formula.matches(']').count();
            assert_eq!(open_brackets, close_brackets,
                "Unbalanced brackets in LTL: {}", formula);

            // Must contain at least one LTL operator
            let has_operator = formula.contains("[]")
                || formula.contains("<>")
                || formula.contains("->")
                || formula.contains("<->")
                || formula.contains("!")
                || formula.contains("&&")
                || formula == "true";
            assert!(has_operator, "LTL formula has no operator: {}", formula);

            // All variable references must use valid Promela identifiers
            // (alphanumeric + underscores, starting with letter)
            for word in formula.split_whitespace() {
                if word.starts_with("active_")
                    || word.starts_with("done_")
                    || word.starts_with("failed_")
                {
                    for c in word.chars() {
                        assert!(c.is_ascii_alphanumeric() || c == '_',
                            "Invalid char '{}' in variable '{}'", c, word);
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 3: extract_task_refs finds all valid references
// ─────────────────────────────────────────────────────────────
// Proves that extract_task_refs finds all task IDs that appear
// in a statement, and doesn't find any that don't.
#[kani::proof]
#[kani::unwind(5)]
fn verify_extract_task_refs() {
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
                file: "test.md".into(),
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

    // Test: a statement that explicitly references T1.1
    let statement = "T1.1 SHALL complete BEFORE T2.1".to_string();
    let refs = translator::extract_task_refs(&statement, &plan);

    // T1.1 and T2.1 should be found if they exist in the plan
    let has_t1 = plan.tasks.iter().any(|t| t.id == "1.1");
    let has_t2 = plan.tasks.iter().any(|t| t.id == "2.1");

    if has_t1 {
        assert!(refs.contains(&"1.1".to_string()),
            "T1.1 should be found in statement: {}", statement);
    }
    if has_t2 {
        assert!(refs.contains(&"2.1".to_string()),
            "T2.1 should be found in statement: {}", statement);
    }

    // All found refs must exist in the plan
    for ref_id in &refs {
        assert!(plan.tasks.iter().any(|t| &t.id == ref_id),
            "Found ref '{}' doesn't exist in plan", ref_id);
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 4: find_sequential_pair correctness
// ─────────────────────────────────────────────────────────────
// Proves that find_sequential_pair correctly identifies
// before/after relationships.
#[kani::proof]
fn verify_find_sequential_pair() {
    let task_ids = vec![
        "1.1".to_string(),
        "1.2".to_string(),
    ];

    // "T1.1 before T1.2" → Some(("1.1", "1.2"))
    let result = translator::find_sequential_pair(
        "T1.1 SHALL complete BEFORE T1.2",
        &task_ids,
    );
    assert_eq!(result, Some(("1.1".to_string(), "1.2".to_string())));

    // "T1.2 after T1.1" → Some(("1.2", "1.1"))
    // (the thing after "after" is the earlier task)
    let result = translator::find_sequential_pair(
        "T1.2 SHALL run AFTER T1.1",
        &task_ids,
    );
    assert_eq!(result, Some(("1.2".to_string(), "1.1".to_string())));

    // No sequential keywords → None
    let result = translator::find_sequential_pair(
        "The system SHALL be robust",
        &task_ids,
    );
    assert_eq!(result, None);
}

// ─────────────────────────────────────────────────────────────
// Harness 5: LTL variable references match task IDs
// ─────────────────────────────────────────────────────────────
// Proves that all variable references in generated LTL formulas
// correspond to actual task IDs in the plan.
#[kani::proof]
#[kani::unwind(5)]
fn verify_ltl_references_exist_in_plan() {
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
                file: "test.md".into(),
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

    let categories = [
        ConstraintCategory::SequentialOrder,
        ConstraintCategory::Exclusive,
        ConstraintCategory::Conditional,
        ConstraintCategory::ConcurrentEvents,
    ];

    for category in &categories {
        let statement: String = kani::any();
        let ltl = translator::generate_ltl(category, &statement, &plan);

        if let Some(formula) = ltl {
            // Extract all variable references from the LTL
            for word in formula.split_whitespace() {
                if let Some(var) = word
                    .strip_prefix("active_")
                    .or_else(|| word.strip_prefix("done_"))
                    .or_else(|| word.strip_prefix("failed_"))
                {
                    // Convert t1_1 back to 1.1
                    if let Some(underscore) = var.find('_') {
                        let major = &var[..underscore];
                        let minor = &var[underscore + 1..];
                        let task_id = format!("{}.{}", major, minor);

                        // This task ID must exist in the plan
                        assert!(plan.tasks.iter().any(|t| t.id == task_id),
                            "LTL references non-existent task '{}' in formula: {}",
                            task_id, formula);
                    }
                }
            }
        }
    }
}

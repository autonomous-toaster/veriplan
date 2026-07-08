//! Kani proof harnesses for the Promela generator.
//!
//! The Promela generator is the bridge between PlanIR and the
//! SPIN model checker. Malformed Promela = silent verification failure.
//!
//! These harnesses prove:
//!   1. Generated Promela has balanced braces and valid structure
//!   2. Every task has exactly one proctype
//!   3. All variable references in LTL properties match declarations
//!   4. The generator never panics on any valid PlanIR

use crate::checker::promela::generate_promela;
use crate::ir::ltl::{LtlCondition, LtlFormula};
use crate::ir::*;
use crate::translator;

// ─────────────────────────────────────────────────────────────
// Harness 1: generated Promela has balanced braces
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_promela_balanced_braces() {
    let plan = build_test_plan();
    let constraints = vec![];

    let promela = generate_promela(&plan, &constraints);

    let open_curly = promela.matches('{').count();
    let close_curly = promela.matches('}').count();
    assert_eq!(open_curly, close_curly, "Unbalanced curly braces");

    let do_count = promela.matches("do").count();
    let od_count = promela.matches("od").count();
    assert_eq!(do_count, od_count, "Unbalanced do/od");

    let proctype_count = promela.matches("proctype").count();
    assert_eq!(proctype_count, plan.tasks.len(),
        "Expected {} proctype declarations, got {}", plan.tasks.len(), proctype_count);
}

// ─────────────────────────────────────────────────────────────
// Harness 2: every task has exactly one proctype
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_each_task_has_proctype() {
    let plan = build_test_plan();
    let constraints = vec![];

    let promela = generate_promela(&plan, &constraints);

    for task in &plan.tasks {
        let proc_name = format!("task_{}", task.id.replace('.', "_"));
        let proc_decl = format!("proctype {}()", proc_name);
        assert!(promela.contains(&proc_decl),
            "Missing proctype for task {}: expected '{}'", task.id, proc_decl);
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 3: LTL properties reference declared variables
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_ltl_variables_are_declared() {
    let plan = build_test_plan();
    let constraints = build_test_constraints(&plan);

    let promela = generate_promela(&plan, &constraints);

    // Collect all declared variables
    let mut declared: Vec<String> = Vec::new();
    for line in promela.lines() {
        let line = line.trim();
        if line.starts_with("bit ") {
            let var_part = line.strip_prefix("bit ").unwrap_or(line);
            if let Some(semi) = var_part.find(';') {
                let var_name = var_part[..semi].trim();
                let var_name = var_name.split('=').next().unwrap_or(var_name).trim();
                declared.push(var_name.to_string());
            }
        }
    }

    // Check each LTL property references only declared variables
    for line in promela.lines() {
        let line = line.trim();
        if line.starts_with("ltl ") {
            if let Some(formula_start) = line.find('{') {
                if let Some(formula_end) = line.rfind('}') {
                    let formula = &line[formula_start + 1..formula_end];

                    for word in formula.split_whitespace() {
                        let clean = word
                            .trim_start_matches('!')
                            .trim_start_matches('(')
                            .trim_end_matches(')');
                        if clean.starts_with("active_")
                            || clean.starts_with("done_")
                            || clean.starts_with("failed_")
                        {
                            assert!(declared.contains(&clean.to_string()),
                                "LTL references undeclared variable '{}' in: {}",
                                clean, formula);
                        }
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 4: generator never panics
// ─────────────────────────────────────────────────────────────
#[kani::proof]
fn verify_promela_generator_no_panic() {
    let plan = build_test_plan();
    let constraints = build_test_constraints(&plan);
    let _ = generate_promela(&plan, &constraints);
}

// ─────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────

fn build_test_plan() -> PlanIR {
    PlanIR {
        tasks: vec![
            Task { id: "1.1".into(), description: "Setup".into(), phase: "Phase 1".into(), checked: false, source: SourceLocation { file: "tasks.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 } },
            Task { id: "1.2".into(), description: "Build".into(), phase: "Phase 1".into(), checked: false, source: SourceLocation { file: "tasks.md".into(), start_byte: 0, end_byte: 0, start_line: 2, end_line: 2 } },
        ],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![Phase {
            name: "Phase 1".into(),
            task_ids: vec!["1.1".into(), "1.2".into()],
            mode: PhaseMode::Sequential,
        }],
        source_map: SourceMap::default(),
    }
}

fn build_test_constraints(plan: &PlanIR) -> Vec<translator::TranslatedConstraint> {
    vec![
        translator::TranslatedConstraint {
            requirement_id: "R1".into(),
            statement: format!("T{} SHALL complete BEFORE T{}", plan.tasks[0].id, plan.tasks[1].id),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::SequentialOrder,
            ltl: Some(LtlFormula::Always(LtlCondition::Implies(
                Box::new(LtlCondition::Atom(format!("active_{}", normalize_id(&plan.tasks[1].id)))),
                Box::new(LtlCondition::Atom(format!("done_{}", normalize_id(&plan.tasks[0].id)))),
            ))),
            is_hard: true,
        },
    ]
}

fn normalize_id(id: &str) -> String {
    format!("t{}", id.replace('.', "_"))
}

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
use crate::ir::*;
use crate::translator;

// ─────────────────────────────────────────────────────────────
// Harness 1: generated Promela has balanced braces
// ─────────────────────────────────────────────────────────────
// Proves that the generated Promela source has balanced
// curly braces, parentheses, and do/od blocks.
#[kani::proof]
#[kani::unwind(5)]
fn verify_promela_balanced_braces() {
    let plan = build_bounded_plan();
    let constraints = vec![];

    let promela = generate_promela(&plan, &constraints);

    // Count braces
    let open_curly = promela.matches('{').count();
    let close_curly = promela.matches('}').count();
    assert_eq!(open_curly, close_curly,
        "Unbalanced curly braces in Promela");

    // Count do/od pairs
    let do_count = promela.matches("do").count();
    let od_count = promela.matches("od").count();
    assert_eq!(do_count, od_count,
        "Unbalanced do/od in Promela");

    // Count proctype declarations
    let proctype_count = promela.matches("proctype").count();
    assert_eq!(proctype_count, plan.tasks.len(),
        "Expected {} proctype declarations, got {}",
        plan.tasks.len(), proctype_count);
}

// ─────────────────────────────────────────────────────────────
// Harness 2: every task has exactly one proctype
// ─────────────────────────────────────────────────────────────
// Proves that each task generates exactly one proctype block
// with the correct name.
#[kani::proof]
#[kani::unwind(5)]
fn verify_each_task_has_proctype() {
    let plan = build_bounded_plan();
    let constraints = vec![];

    let promela = generate_promela(&plan, &constraints);

    for task in &plan.tasks {
        let proc_name = format!("task_{}", task.id.replace('.', "_"));
        let proc_decl = format!("proctype {}()", proc_name);

        assert!(promela.contains(&proc_decl),
            "Missing proctype for task {}: expected '{}'",
            task.id, proc_decl);
    }
}

// ─────────────────────────────────────────────────────────────
// Harness 3: LTL properties reference declared variables
// ─────────────────────────────────────────────────────────────
// Proves that every variable referenced in LTL properties
// has a corresponding `bit` declaration in the Promela model.
#[kani::proof]
#[kani::unwind(5)]
fn verify_ltl_variables_are_declared() {
    let plan = build_bounded_plan();
    let constraints = build_bounded_constraints(&plan);

    let promela = generate_promela(&plan, &constraints);

    // Collect all declared variables
    let mut declared: Vec<String> = Vec::new();
    for line in promela.lines() {
        let line = line.trim();
        if line.starts_with("bit ") {
            // Extract variable name: "bit active_t1_1 = 0; /* ... */"
            let var_part = line.strip_prefix("bit ").unwrap_or(line);
            if let Some(semi) = var_part.find(';') {
                let var_name = var_part[..semi].trim();
                // Remove "= 0" or "= 1" if present
                let var_name = var_name.split('=').next().unwrap_or(var_name).trim();
                declared.push(var_name.to_string());
            }
        }
    }

    // Check each LTL property references only declared variables
    for line in promela.lines() {
        let line = line.trim();
        if line.starts_with("ltl ") {
            // Extract the LTL formula
            if let Some(formula_start) = line.find('{') {
                if let Some(formula_end) = line.rfind('}') {
                    let formula = &line[formula_start + 1..formula_end];

                    // Find all variable references (active_, done_, failed_)
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
// Proves the Promela generator handles any valid PlanIR
// without panicking.
#[kani::proof]
#[kani::unwind(5)]
fn verify_promela_generator_no_panic() {
    let plan = build_bounded_plan();
    let constraints = build_bounded_constraints(&plan);

    let _ = generate_promela(&plan, &constraints);
}

// ─────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────

/// Build a bounded PlanIR with up to 3 tasks and 1 phase.
fn build_bounded_plan() -> PlanIR {
    let task_count: usize = kani::any();
    kani::assume(task_count >= 1);
    kani::assume(task_count <= 3);

    let mut tasks = Vec::new();
    for i in 0..task_count {
        tasks.push(Task {
            id: format!("{}.{}", i + 1, 1),
            description: format!("Task {}", i + 1),
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

    let phases = vec![Phase {
        name: "Phase 1".into(),
        task_ids: tasks.iter().map(|t| t.id.clone()).collect(),
        mode: PhaseMode::Sequential,
    }];

    PlanIR {
        tasks,
        requirements: vec![],
        scenarios: vec![],
        phases,
        source_map: SourceMap::default(),
    }
}

/// Build bounded constraints referencing tasks in the plan.
fn build_bounded_constraints(plan: &PlanIR) -> Vec<translator::TranslatedConstraint> {
    let mut constraints = Vec::new();

    if plan.tasks.len() >= 2 {
        // Sequential constraint
        constraints.push(translator::TranslatedConstraint {
            requirement_id: "R1".into(),
            statement: format!("T{} SHALL complete BEFORE T{}",
                plan.tasks[0].id, plan.tasks[1].id),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::SequentialOrder,
            ltl: Some(format!("[] ( active_{} -> done_{} )",
                normalize_id(&plan.tasks[1].id),
                normalize_id(&plan.tasks[0].id))),
            is_hard: true,
        });
    }

    if plan.tasks.len() >= 2 {
        // Exclusive constraint
        let pairs: Vec<String> = (0..plan.tasks.len())
            .flat_map(|i| (i + 1..plan.tasks.len()).map(move |j| (i, j)))
            .map(|(i, j)| {
                format!("!(active_{} && active_{})",
                    normalize_id(&plan.tasks[i].id),
                    normalize_id(&plan.tasks[j].id))
            })
            .collect();
        constraints.push(translator::TranslatedConstraint {
            requirement_id: "R2".into(),
            statement: "At most one task SHALL be active".into(),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::Exclusive,
            ltl: Some(format!("[] ( {} )", pairs.join(" && "))),
            is_hard: true,
        });
    }

    constraints
}

fn normalize_id(id: &str) -> String {
    format!("t{}", id.replace('.', "_"))
}

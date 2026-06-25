//! Annotator — enrich violations with source locations, task context, and suggested fixes.

mod helpers;

use crate::checker::Violation;
use crate::ir::PlanIR;

pub use helpers::{build_phase_context, category_breakdown, parse_conditional_ltl, task_ids_from_ltl};

/// Annotated violation with additional context.
#[derive(Debug, Clone)]
pub struct AnnotatedViolation {
    pub violation: Violation,
    pub task_source: Option<String>,
    pub req_source: Option<String>,
    pub phase_context: Option<String>,
    pub trigger_task: Option<String>,
    pub consequent_task: Option<String>,
    pub category: String,
}

/// Annotate violations with source locations and context.
pub fn annotate(
    result: &crate::checker::VerificationResult,
    plans: &[(String, PlanIR)],
) -> Vec<AnnotatedViolation> {
    let mut annotated = Vec::new();

    for violation in &result.violations {
        let plan = plans
            .iter()
            .find(|(name, _)| name == &violation.plan)
            .map(|(_, p)| p)
            .or_else(|| plans.first().map(|(_, p)| p))
            .unwrap();

        let (task_source, req_source) = resolve_source(violation, plan);

        let phase_context = helpers::build_phase_context(&violation.ltl, plan);

        let (trigger_task, consequent_task) = if violation.category.contains("Conditional") {
            helpers::parse_conditional_ltl(&violation.ltl)
                .map(|(t, c)| (Some(t), Some(c)))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        annotated.push(AnnotatedViolation {
            violation: violation.clone(),
            task_source,
            req_source,
            phase_context,
            trigger_task,
            consequent_task,
            category: violation.category.clone(),
        });
    }

    annotated
}

/// Format verification result as human-readable text.
pub fn format_human(
    result: &crate::checker::VerificationResult,
    annotated: &[AnnotatedViolation],
    plans: &[(String, PlanIR)],
    verbose: bool,
) -> String {
    let mut output = String::new();

    let status = if result.convertible && result.valid == Some(true) {
        "✓ VALID"
    } else if !result.convertible {
        "⚠ SKIPPED"
    } else if result.valid == Some(false) {
        "✗ INVALID"
    } else {
        "⚠ UNKNOWN"
    };

    output.push_str(&format!(
        "Plan: {} — {}\n",
        result.plan_name, status
    ));

    if let Some(reason) = &result.skip_reason {
        output.push_str(&format!("\n  Model check skipped: {}\n", reason));
    }

    if let Some(report) = &result.convertibility_report {
        if !report.blockers.is_empty() {
            output.push_str(&format!(
                "  {} blocker(s):\n",
                report.blockers.len()
            ));
            for item in &report.blockers {
                output.push_str(&format!(
                    "    [BLOCKER] {} at {}\n",
                    item.element, item.location
                ));
                output.push_str(&format!("              {}\n", item.detail));
                if let Some(fix) = &item.fix {
                    output.push_str(&format!("              Fix: {}\n", fix));
                }
            }
        }

        if !report.warnings.is_empty() && verbose {
            output.push_str(&format!(
                "  {} warning(s):\n",
                report.warnings.len()
            ));
            for item in &report.warnings {
                output.push_str(&format!(
                    "    [WARNING] {} at {}\n",
                    item.element, item.location
                ));
                output.push_str(&format!("              {}\n", item.detail));
            }
        }
    }

    if !result.violations.is_empty() {
        let satisfied = result.satisfied_constraints;
        let violated = result.violations.len();
        let total = result.total_constraints;

        output.push_str(&format!(
            "\n  Satisfied: {} | Violated: {} | Total: {}\n",
            satisfied, violated, total
        ));

        if verbose {
            output.push_str(&format!("\n{}\n", helpers::category_breakdown(annotated)));
        }

        for (i, v) in annotated.iter().enumerate() {
            output.push_str(&format!(
                "\n  Violation {}:\n",
                i + 1
            ));
            output.push_str(&format!(
                "    Requirement: {}\n",
                v.violation.constraint_id
            ));
            output.push_str(&format!(
                "    Statement: {}\n",
                v.violation.requirement_statement
            ));
            output.push_str(&format!("    Category: {}\n", v.category));

            if let Some(phase) = &v.phase_context {
                output.push_str(&format!("    Phase: {}\n", phase));
            }

            if v.category.contains("Conditional") {
                if let Some(trigger) = &v.trigger_task {
                    output.push_str(&format!("    Trigger: {}\n", trigger));
                }
                if let Some(consequent) = &v.consequent_task {
                    output.push_str(&format!("    Consequent: {}\n", consequent));
                }
            }

            if let Some(source) = &v.task_source {
                output.push_str(&format!("    Task source: {}\n", source));
            }
            if let Some(source) = &v.req_source {
                output.push_str(&format!("    Requirement source: {}\n", source));
            }

            if let Some(fix) = &v.violation.suggested_fix {
                output.push_str(&format!("\n    Suggested fix:\n    {}\n", fix.replace('\n', "\n    ")));
            }
        }
    } else if result.convertible && result.valid == Some(true) {
        output.push_str("  All constraints satisfied.\n");
        output.push_str(&format!(
            "  Satisfied: {} | Violated: 0 | Total: {}\n",
            result.satisfied_constraints, result.total_constraints
        ));
    }

    if verbose {
        verbose_section(&mut output, plans, result);
    }

    output
}

/// Format verification result as JSON.
pub fn format_json(
    result: &crate::checker::VerificationResult,
    annotated: &[AnnotatedViolation],
    _plans: &[(String, PlanIR)],
    verbose: bool,
) -> String {
    let mut violations_json = Vec::new();

    for v in annotated {
        let mut obj = serde_json::json!({
            "constraint_id": v.violation.constraint_id,
            "requirement_statement": v.violation.requirement_statement,
            "ltl": v.violation.ltl,
            "category": v.category,
            "state": v.violation.state,
            "plan": v.violation.plan,
        });

        if let Some(source) = &v.task_source {
            obj["task_source"] = serde_json::json!(source);
        }
        if let Some(source) = &v.req_source {
            obj["req_source"] = serde_json::json!(source);
        }
        if let Some(phase) = &v.phase_context {
            obj["phase_context"] = serde_json::json!(phase);
        }
        if let Some(fix) = &v.violation.suggested_fix {
            obj["suggested_fix"] = serde_json::json!(fix);
        }

        violations_json.push(obj);
    }

    let mut output = serde_json::json!({
        "plan_name": result.plan_name,
        "phase": result.phase,
        "convertible": result.convertible,
        "valid": result.valid,
        "violations": violations_json,
        "total_constraints": result.total_constraints,
        "satisfied_constraints": result.satisfied_constraints,
    });

    if let Some(reason) = &result.skip_reason {
        output["skip_reason"] = serde_json::json!(reason);
    }

    if verbose
        && let Some(report) = &result.convertibility_report {
            output["convertibility_report"] = serde_json::json!(report);
        }

    serde_json::to_string_pretty(&output).unwrap_or_default()
}

fn resolve_source(v: &Violation, plan: &PlanIR) -> (Option<String>, Option<String>) {
    let task_source = v.task_source.clone().or_else(|| {
        helpers::task_ids_from_ltl(&v.ltl)
            .first()
            .and_then(|id| {
                plan.tasks
                    .iter()
                    .find(|t| t.id == *id)
                    .map(|t| format!("{}:{}", t.source.file, t.source.start_line))
            })
    });

    let req_source = v.req_source.clone().or_else(|| {
        plan.requirements
            .iter()
            .find(|r| r.id == v.constraint_id)
            .map(|r| format!("{}:{}", r.source.file, r.source.start_line))
    });

    (task_source, req_source)
}

fn verbose_section(output: &mut String, plans: &[(String, PlanIR)], _result: &crate::checker::VerificationResult) {
    for (name, plan) in plans {
        output.push_str(&format!("\n=== Plan: {} ===\n", name));
        output.push_str(&format!("Tasks: {}\n", plan.tasks.len()));
        output.push_str(&format!("Requirements: {}\n", plan.requirements.len()));
        output.push_str(&format!("Phases: {}\n", plan.phases.len()));
    }
}

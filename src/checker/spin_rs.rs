use crate::checker::{ConstraintSummary, VerificationResult, Violation};
use crate::ir::*;
use crate::translator;

/// Default state budget for the in-process engine.
///
/// The engine has no wall-clock deadline, so a model whose state space does not close runs
/// without bound. A budget makes the run terminate; while the engine cannot report that it
/// stopped early, reaching the budget is treated as 'unchecked' rather than 'passed'.
fn default_state_budget() -> usize {
    std::env::var("VERIPLAN_MAX_STATES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000)
}

pub(crate) fn run_spin_rs_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let promela = super::promela::generate_promela(plan, constraints);
    let budget = default_state_budget();

    let config = spin_rs::CheckerConfig {
        max_states: budget,
        ..Default::default()
    };

    let check_result = match spin_rs::verify_with_config(&promela, &config) {
        Ok(r) => r,
        Err(e) => {
            // The engine refuses models it cannot explore or evaluate. That is
            // 'unverified', never 'invalid' and never 'valid'.
            return VerificationResult {
                plan_name: plan_name.to_string(),
                phase: "model_check".into(),
                convertible: true,
                convertibility_report: Some(conv_report),
                valid: None,
                violations: vec![],
                total_constraints: constraints.len(),
                satisfied_constraints: 0,
                unmodelled_constraints: constraints.iter().filter(|c| !c.modelled()).count(),
                unchecked_constraints: constraints.iter().filter(|c| c.modelled()).count(),
                backend: "spin-rs".into(),
                constraints_summary: vec![],
                skip_reason: Some(format!("spin-rs verification error: {}", e)),
            };
        }
    };

    // Without a truncation signal from the engine, a search that consumed its whole budget
    // is indistinguishable from one that closed the space. Treat it as unchecked.
    let budget_exhausted = check_result.states_explored >= budget;

    let mut violated_indices: Vec<usize> = Vec::new();
    for v in &check_result.violations {
        if let Some(idx) = parse_property_index(&v.property_name) {
            violated_indices.push(idx);
        }
    }
    violated_indices.sort();
    violated_indices.dedup();

    let mut constraints_summary = Vec::new();
    let mut violations = Vec::new();
    let mut satisfied = 0usize;
    let mut unmodelled = 0usize;
    let mut unchecked = 0usize;
    let mut claim_index = 0usize;

    for c in constraints {
        let category = format!("{:?}", c.category);
        if !c.modelled() {
            unmodelled += 1;
            constraints_summary.push(ConstraintSummary {
                requirement_id: c.requirement_id.clone(),
                statement: c.statement.clone(),
                category,
                satisfied: false,
                unchecked: false,
                modelled: false,
                unmodelled_reason: c.unmodelled_reason.clone(),
            });
            continue;
        }

        let idx = claim_index;
        claim_index += 1;
        let is_violated = violated_indices.contains(&idx);
        let is_unchecked = !is_violated && budget_exhausted;

        if is_violated {
            let spin_violation = check_result
                .violations
                .iter()
                .find(|v| parse_property_index(&v.property_name) == Some(idx));
            violations.push(Violation {
                constraint_id: c.requirement_id.clone(),
                requirement_statement: c.statement.clone(),
                ltl: c.ltl_string(),
                category: category.clone(),
                state: spin_violation
                    .map(|v| v.description.clone())
                    .unwrap_or_else(|| format!("(violated in property p{})", idx)),
                task_source: None,
                req_source: None,
                suggested_fix: super::bfs::suggest_fix(
                    &c.category,
                    c.ltl_string().as_str(),
                    &c.requirement_id,
                    &c.statement,
                ),
                plan: plan_name.to_string(),
                kind: crate::ir::kind_of(&category),
                op: crate::ir::Op::ReplaceBody,
            });
        } else if is_unchecked {
            unchecked += 1;
        } else {
            satisfied += 1;
        }

        constraints_summary.push(ConstraintSummary {
            requirement_id: c.requirement_id.clone(),
            statement: c.statement.clone(),
            category,
            satisfied: !is_violated && !is_unchecked,
            unchecked: is_unchecked,
            modelled: true,
            unmodelled_reason: None,
        });
    }

    let nothing_proved = satisfied == 0 && violations.is_empty();
    let (valid, skip_reason) = if nothing_proved && !constraints.is_empty() {
        (
            None,
            Some(format!(
                "no constraint was proved or refuted ({} unmodelled, {} unchecked, {} states explored)",
                unmodelled, unchecked, check_result.states_explored
            )),
        )
    } else {
        (Some(violations.is_empty()), None)
    };

    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid,
        violations,
        total_constraints: constraints.len(),
        satisfied_constraints: satisfied,
        unmodelled_constraints: unmodelled,
        unchecked_constraints: unchecked,
        backend: "spin-rs".into(),
        constraints_summary,
        skip_reason,
    }
}

fn parse_property_index(name: &str) -> Option<usize> {
    name.trim().strip_prefix('p')?.parse().ok()
}

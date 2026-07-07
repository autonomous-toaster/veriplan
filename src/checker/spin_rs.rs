use crate::checker::{ConstraintSummary, VerificationResult, Violation};
use crate::ir::*;
use crate::translator;

/// Run model checking using the spin-rs library (in-process, no external binary).
pub(crate) fn run_spin_rs_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let promela = super::promela::generate_promela(plan, constraints);

    // Run spin-rs verification
    let check_result = match spin_rs::verify(&promela) {
        Ok(r) => r,
        Err(e) => {
            return VerificationResult {
                plan_name: plan_name.to_string(),
                phase: "model_check".into(),
                convertible: true,
                convertibility_report: Some(conv_report),
                valid: None,
                violations: vec![],
                total_constraints: 0,
                satisfied_constraints: 0,
                constraints_summary: vec![],
                skip_reason: Some(format!("spin-rs verification error: {}", e)),
            };
        }
    };

    // Build a set of violated property indices from spin-rs results
    // spin-rs labels LTL properties as "p0", "p1", etc. matching our Promela generation
    let mut violated_indices: Vec<usize> = Vec::new();
    for v in &check_result.violations {
        if let Some(idx) = parse_property_index(&v.property_name) {
            violated_indices.push(idx);
        }
    }
    violated_indices.sort();
    violated_indices.dedup();

    // Build constraints_summary and violations from formalizable constraints
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();
    let mut constraints_summary = Vec::new();
    let mut violations = Vec::new();
    let mut satisfied = 0usize;

    for (i, c) in formalizable.iter().enumerate() {
        let is_violated = violated_indices.contains(&i);
        constraints_summary.push(ConstraintSummary {
            requirement_id: c.requirement_id.clone(),
            statement: c.statement.clone(),
            category: format!("{:?}", c.category),
            satisfied: !is_violated,
            unchecked: false,
        });

        if is_violated {
            // Find the matching spin-rs violation for details
            let spin_violation = check_result
                .violations
                .iter()
                .find(|v| parse_property_index(&v.property_name) == Some(i));

            violations.push(Violation {
                constraint_id: c.requirement_id.clone(),
                requirement_statement: c.statement.clone(),
                ltl: c.ltl.clone().unwrap_or_default(),
                category: format!("{:?}", c.category),
                state: spin_violation
                    .map(|v| v.description.clone())
                    .unwrap_or_else(|| format!("(violated in property p{})", i)),
                task_source: None,
                req_source: None,
                suggested_fix: super::bfs::suggest_fix(
                    &c.category,
                    c.ltl.as_deref().unwrap_or(""),
                    &c.requirement_id,
                    &c.statement,
                ),
                plan: plan_name.to_string(),
            });
        } else {
            satisfied += 1;
        }
    }

    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid: Some(violations.is_empty()),
        violations,
        total_constraints: formalizable.len(),
        satisfied_constraints: satisfied,
        constraints_summary,
        skip_reason: None,
    }
}

/// Parse a property index from a spin-rs property name like "p0", "p1", etc.
fn parse_property_index(name: &str) -> Option<usize> {
    let name = name.trim();
    if let Some(digits) = name.strip_prefix('p') {
        digits.parse::<usize>().ok()
    } else {
        None
    }
}

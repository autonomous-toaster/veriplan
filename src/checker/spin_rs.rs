use crate::checker::{ConstraintSummary, VerificationResult, Violation};
use crate::ir::*;
use crate::translator;

/// State budget for the in-process engine when the caller does not set one.
pub(crate) const DEFAULT_STATE_BUDGET: usize = 200_000;

/// Read the state budget a caller set.
///
/// A value the caller set is never replaced silently. Zero is the one value that means "do not
/// run", and a value that is not a number is a caller error: either way the run says so instead of
/// running with a positive budget nobody asked for. The engine divides the budget across the
/// claims it checks, so the per-claim share is reported with the claim rather than inferred here.
pub(crate) fn state_budget_from(raw: Option<&str>) -> anyhow::Result<usize> {
    match raw {
        None => Ok(DEFAULT_STATE_BUDGET),
        Some(text) => {
            let parsed: usize = text.trim().parse().map_err(|_| {
                anyhow::anyhow!("VERIPLAN_MAX_STATES is not a state count: {text:?}")
            })?;
            if parsed == 0 {
                anyhow::bail!(
                    "VERIPLAN_MAX_STATES is 0, which no search can run under; unset it for the default of {DEFAULT_STATE_BUDGET}"
                );
            }
            Ok(parsed)
        }
    }
}

/// The state budget the in-process engine is given, from the environment.
fn default_state_budget() -> anyhow::Result<usize> {
    state_budget_from(std::env::var("VERIPLAN_MAX_STATES").ok().as_deref())
}

pub(crate) fn run_spin_rs_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let promela = super::promela::generate_promela(plan, constraints);
    let budget = match default_state_budget() {
        Ok(budget) => budget,
        Err(e) => {
            // A budget the caller set and we cannot honour is a refusal, not a reason to pick one.
            return VerificationResult {
                plan_name: plan_name.to_string(),
                phase: "model_check".into(),
                convertible: true,
                convertibility_report: Some(conv_report),
                valid: None,
                violations: vec![],
                total_constraints: constraints.len(),
                satisfied_constraints: 0,
                unmodelled_constraints: constraints
                    .iter()
                    .filter(|c| !c.modelled() && !c.historical())
                    .count(),
                unchecked_constraints: constraints.iter().filter(|c| c.modelled()).count(),
                historical_constraints: constraints.iter().filter(|c| c.historical()).count(),
                backend: "spin-rs".into(),
                constraints_summary: vec![],
                skip_reason: Some(format!("{e}")),
            };
        }
    };

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
                unmodelled_constraints: constraints
                    .iter()
                    .filter(|c| !c.modelled() && !c.historical())
                    .count(),
                unchecked_constraints: constraints.iter().filter(|c| c.modelled()).count(),
                historical_constraints: constraints.iter().filter(|c| c.historical()).count(),
                backend: "spin-rs".into(),
                constraints_summary: vec![],
                skip_reason: Some(format!("spin-rs verification error: {}", e)),
            };
        }
    };

    // The engine reports properties it could not decide. A budget that stopped the search,
    // an unsupported shape, or an unparsable formula lands there, and none of them licenses a
    // success claim: errors == 0 with a non-empty unverified list is not a clean run.
    let model_entry = check_result
        .unverified
        .iter()
        .find(|u| u.property_name == "model");

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
    let mut historical = 0usize;
    let mut unchecked = 0usize;
    let mut claim_index = 0usize;

    for c in constraints {
        let category = format!("{:?}", c.category);
        if !c.modelled() {
            if c.historical() {
                historical += 1;
            } else {
                unmodelled += 1;
            }
            constraints_summary.push(ConstraintSummary {
                requirement_id: c.requirement_id.clone(),
                statement: c.statement.clone(),
                category,
                satisfied: false,
                unchecked: false,
                modelled: false,
                unchecked_reason: None,
                states_used: None,
                states_allowed: None,
                historical: c.historical.is_some(),
                historical_owner: c.historical.clone(),
                unmodelled_reason: c.unmodelled_reason.clone(),
            });
            continue;
        }

        let idx = claim_index;
        claim_index += 1;
        let label = format!("p{idx}");
        let is_violated = violated_indices.contains(&idx);
        let claim_entry = check_result
            .unverified
            .iter()
            .find(|u| u.property_name == label);
        // A claim may be undecided because of its own limitation or because the whole run
        // stopped. Name the reason either way: "unchecked" alone tells a caller nothing.
        //
        // A refuted claim is decided even when the engine also reports it as not fully
        // explored - the traversal found a counterexample, and what it would have done with
        // the rest of its budget cannot change that. Without this priority the per-claim
        // summary says "unchecked" for claims the total counts as violated, so the report
        // contradicts its own arithmetic.
        let undecided_entry = claim_entry.or(model_entry);
        let status = claim_status(is_violated, undecided_entry);

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
                    .unwrap_or_else(|| format!("(violated in property {label})")),
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
        } else if status.unchecked {
            unchecked += 1;
        } else {
            satisfied += 1;
        }

        constraints_summary.push(ConstraintSummary {
            requirement_id: c.requirement_id.clone(),
            statement: c.statement.clone(),
            category,
            satisfied: status.satisfied,
            unchecked: status.unchecked,
            modelled: true,
            unchecked_reason: status.reason,
            // The engine named this claim, so its own numbers are the ones to report. A decided
            // claim carries none: the whole-model entry is about the run, not about this claim.
            states_used: claim_entry.map(|u| u.states_used),
            states_allowed: claim_entry.and_then(|u| u.budget),
            historical: false,
            historical_owner: None,
            unmodelled_reason: None,
        });
    }

    let nothing_proved = satisfied == 0 && violations.is_empty();
    let (valid, skip_reason) = if nothing_proved && !constraints.is_empty() {
        (
            None,
            Some(format!(
                "no constraint was proved or refuted ({} unmodelled, {} unchecked{}, {} states explored under a run budget of {budget} shared across the claims checked)",
                unmodelled,
                unchecked,
                check_result
                    .unverified
                    .first()
                    .map(|u| format!(" — {}", u.kind.as_str()))
                    .unwrap_or_default(),
                check_result.states_explored
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
        historical_constraints: historical,
        backend: "spin-rs".into(),
        constraints_summary,
        skip_reason,
    }
}

/// A modelled claim's verdict, derived once so the tally and the per-claim summary cannot
/// disagree.
///
/// A refuted claim is decided even when the engine also reports it as not fully explored: the
/// traversal found a counterexample, and the rest of its budget cannot take that back. Anything
/// else that carries no verdict is unchecked, and says why.
#[derive(Debug, PartialEq, Eq)]
struct ClaimStatus {
    satisfied: bool,
    unchecked: bool,
    reason: Option<String>,
}

fn claim_status(is_violated: bool, undecided: Option<&spin_rs::Unverified>) -> ClaimStatus {
    if is_violated {
        return ClaimStatus {
            satisfied: false,
            unchecked: false,
            reason: None,
        };
    }
    match undecided {
        Some(u) => ClaimStatus {
            satisfied: false,
            unchecked: true,
            reason: Some(format!("{}: {}", u.kind.as_str(), u.reason)),
        },
        None => ClaimStatus {
            satisfied: true,
            unchecked: false,
            reason: None,
        },
    }
}

fn parse_property_index(name: &str) -> Option<usize> {
    name.trim().strip_prefix('p')?.parse().ok()
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; the crate denies these three lints for code that ships.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// A claim the engine both refuted and reported as not fully explored is decided. The
    /// summary must not call it unchecked while the tally counts it as violated: a report that
    /// disagrees with its own arithmetic is worse than one that omits the detail.
    #[test]
    fn a_refuted_claim_is_decided_even_when_the_engine_also_reports_it_unexplored() {
        let unexplored = spin_rs::Unverified {
            property_name: "p3".into(),
            kind: spin_rs::UnverifiedKind::StateBudget,
            reason: "the share was used up".into(),
            states_used: 3_225,
            budget: Some(3_225),
        };
        let status = claim_status(true, Some(&unexplored));
        assert!(!status.satisfied);
        assert!(!status.unchecked, "a refuted claim is not unchecked");
        assert_eq!(status.reason, None);

        let undecided = claim_status(false, Some(&unexplored));
        assert!(!undecided.satisfied);
        assert!(undecided.unchecked);
        assert!(undecided.reason.unwrap().starts_with("state_budget: "));

        let satisfied = claim_status(false, None);
        assert!(satisfied.satisfied);
        assert!(!satisfied.unchecked);
    }

    /// The tags veriplan relays to its callers are the engine's own stable strings. The
    /// dependency has agreed to announce a change to them; this test is what makes that
    /// agreement enforceable rather than a promise.
    #[test]
    fn the_relayed_kinds_are_the_agreed_tags() {
        use spin_rs::UnverifiedKind::*;
        let tags: Vec<&str> = [
            UnsupportedProperty,
            UnparsableFormula,
            StateBudget,
            DepthBudget,
            EarlyStop,
        ]
        .iter()
        .map(|k| k.as_str())
        .collect();
        assert_eq!(
            tags,
            [
                "unsupported_property",
                "unparsable_formula",
                "state_budget",
                "depth_budget",
                "early_stop"
            ],
            "a tag veriplan relays to its callers changed; update the mapping and tell the engine"
        );
    }
}

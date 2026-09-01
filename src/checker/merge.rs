//! Merging multiple verification results — extracted from checker/mod.rs to
//! keep files under the 550-line file-length gate.

use crate::ir::PlanIR;

use super::{CheckerBackend, VerificationResult, verify};

/// Verify multiple plans and merge the results into a single report.
pub fn verify_all(
    plans: &[(String, PlanIR)],
    no_model: bool,
    pre_commit: bool,
    is_openspec: bool,
    backend: CheckerBackend,
) -> VerificationResult {
    let mut all_results: Vec<VerificationResult> = Vec::new();
    for (name, plan) in plans {
        let result = verify(plan, name, no_model, pre_commit, is_openspec, backend);
        all_results.push(result);
    }
    merge_results(&all_results)
}

/// Merge multiple verification results into a combined report.
pub fn merge_results(results: &[VerificationResult]) -> VerificationResult {
    if results.is_empty() {
        return VerificationResult {
            plan_name: String::new(),
            phase: "full".into(),
            convertible: true,
            convertibility_report: None,
            valid: Some(true),
            violations: vec![],
            total_constraints: 0,
            satisfied_constraints: 0,
            constraints_summary: vec![],
            skip_reason: None,
        };
    }

    if results.len() == 1 {
        return results[0].clone();
    }

    let names: Vec<&str> = results.iter().map(|r| r.plan_name.as_str()).collect();
    let combined_name = names.join(", ");

    // Merge: worst outcome wins
    let all_convertible = results.iter().all(|r| r.convertible);
    let any_invalid = results.iter().any(|r| r.valid == Some(false));
    let any_skipped = results.iter().any(|r| r.skip_reason.is_some());
    let any_valid = results.iter().any(|r| r.valid == Some(true));

    let mut combined = VerificationResult {
        plan_name: combined_name,
        phase: "full".into(),
        convertible: all_convertible,
        convertibility_report: None,
        valid: if !all_convertible {
            None
        } else if any_invalid {
            Some(false)
        } else if any_skipped && !any_valid {
            None
        } else {
            Some(true)
        },
        violations: results.iter().flat_map(|r| r.violations.clone()).collect(),
        total_constraints: results.iter().map(|r| r.total_constraints).sum(),
        satisfied_constraints: results.iter().map(|r| r.satisfied_constraints).sum(),
        constraints_summary: results
            .iter()
            .flat_map(|r| r.constraints_summary.clone())
            .collect(),
        skip_reason: None,
    };

    if any_skipped && !any_valid {
        combined.skip_reason = Some("One or more changes were skipped".into());
    }

    combined
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::Violation;

    fn make_result(
        plan_name: &str,
        convertible: bool,
        valid: Option<bool>,
        violations: usize,
    ) -> VerificationResult {
        VerificationResult {
            plan_name: plan_name.into(),
            phase: "full".into(),
            convertible,
            convertibility_report: None,
            valid,
            violations: (0..violations)
                .map(|i| Violation {
                    constraint_id: format!("R{}", i),
                    requirement_statement: "test".into(),
                    ltl: String::new(),
                    category: String::new(),
                    state: String::new(),
                    task_source: None,
                    req_source: None,
                    suggested_fix: None,
                    plan: plan_name.into(),
                    kind: crate::ir::Kind::ProseOther,
                    op: crate::ir::Op::None,
                })
                .collect(),
            total_constraints: 10,
            satisfied_constraints: 5,
            constraints_summary: vec![],
            skip_reason: None,
        }
    }

    #[test]
    fn test_merge_results_empty() {
        let result = merge_results(&[]);
        assert_eq!(result.total_constraints, 0);
        assert_eq!(result.valid, Some(true));
    }

    #[test]
    fn test_merge_results_single() {
        let r = make_result("change1", true, Some(true), 0);
        let result = merge_results(&[r]);
        assert_eq!(result.plan_name, "change1");
        assert_eq!(result.valid, Some(true));
    }

    #[test]
    fn test_merge_results_all_valid() {
        let r1 = make_result("a", true, Some(true), 0);
        let r2 = make_result("b", true, Some(true), 0);
        let result = merge_results(&[r1, r2]);
        assert_eq!(result.valid, Some(true));
        assert_eq!(result.total_constraints, 20);
    }

    #[test]
    fn test_merge_results_any_invalid() {
        let r1 = make_result("a", true, Some(true), 0);
        let r2 = make_result("b", true, Some(false), 2);
        let result = merge_results(&[r1, r2]);
        assert_eq!(result.valid, Some(false));
        assert_eq!(result.violations.len(), 2);
    }

    #[test]
    fn test_merge_results_not_convertible() {
        let r1 = make_result("a", false, None, 0);
        let r2 = make_result("b", true, Some(true), 0);
        let result = merge_results(&[r1, r2]);
        assert!(!result.convertible);
        assert_eq!(result.valid, None);
    }
}

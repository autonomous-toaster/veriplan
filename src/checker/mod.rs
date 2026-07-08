//! Verifier engine: convertibility check → Promela generation → model checking.
//!
//! Three phases:
//!   1. Convertibility check (Phase 1): validate plan can become a formal model
//!   2. Promela + SPIN (Phase 2a): full SPIN model checking
//!   3. BFS fallback (Phase 2b): built-in explorer when SPIN unavailable
#![allow(dead_code)]

pub(crate) mod checks;
mod convertibility;
pub mod bfs;
pub mod promela;
pub mod spin;
pub mod spin_rs;

pub use convertibility::check_convertibility;

use crate::ir::{ConvertibilityReport, ConvertibilityStatus, PlanIR};
use crate::translator;

/// Result of model checking.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub constraint_id: String,
    pub requirement_statement: String,
    pub ltl: String,
    pub category: String,
    pub state: String,
    pub task_source: Option<String>,
    pub req_source: Option<String>,
    pub suggested_fix: Option<String>,
    /// The plan/change this violation belongs to (used for multi-change output).
    pub plan: String,
}

/// Summary of one checked constraint with pass/fail status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstraintSummary {
    pub requirement_id: String,
    pub statement: String,
    pub category: String,
    pub satisfied: bool,
    pub unchecked: bool,
}

/// Final verification result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationResult {
    pub plan_name: String,
    pub phase: String, // "convertibility", "model_check", "full"
    pub convertible: bool,
    pub convertibility_report: Option<ConvertibilityReport>,
    pub valid: Option<bool>,
    pub violations: Vec<Violation>,
    pub total_constraints: usize,
    pub satisfied_constraints: usize,
    /// If None, no skip reason. If Some(reason), model check was skipped.
    pub skip_reason: Option<String>,
    /// Per-constraint pass/fail summary for display.
    pub constraints_summary: Vec<ConstraintSummary>,
}

/// Checker backend selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckerBackend {
    /// External spin binary (default).
    Spin,
    /// In-process spin-rs library.
    SpinRs,
}

impl std::str::FromStr for CheckerBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "spin" => Ok(CheckerBackend::Spin),
            "spin-rs" => Ok(CheckerBackend::SpinRs),
            other => Err(format!(
                "unknown checker backend '{}'. Supported: spin, spin-rs",
                other
            )),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Phase 2: Model Checking
// ═══════════════════════════════════════════════════════════════

/// Check if SPIN is available on PATH.
pub fn require_spin() -> Result<(), String> {
    match std::process::Command::new("spin").arg("--version").output() {
        Ok(_) => Ok(()),
        Err(_) => Err(
            "SPIN binary not found on PATH. Install spin (brew install spin) and try again.".into(),
        ),
    }
}

/// Run the full verification pipeline (Phase 1 + Phase 2).
pub fn verify(
    plan: &PlanIR,
    plan_name: &str,
    no_model: bool,
    pre_commit: bool,
    is_openspec: bool,
    backend: CheckerBackend,
) -> VerificationResult {
    // Phase 1: Convertibility check
    let (conv_report, updated_plan) = check_convertibility(plan, is_openspec);

    if conv_report.status == ConvertibilityStatus::Blocking {
        return VerificationResult {
            plan_name: plan_name.to_string(),
            phase: if no_model {
                "convertibility".into()
            } else {
                "full".into()
            },
            convertible: false,
            convertibility_report: Some(conv_report),
            valid: None,
            violations: vec![],
            total_constraints: 0,
            satisfied_constraints: 0,
            constraints_summary: vec![],
            skip_reason: Some("Convertibility check failed".into()),
        };
    }

    if no_model {
        // Stop after convertibility check
        let warnings_count = conv_report.warnings.len();
        let _info_count = conv_report.info.len();
        return VerificationResult {
            plan_name: plan_name.to_string(),
            phase: "convertibility".into(),
            convertible: true,
            convertibility_report: Some(conv_report),
            valid: Some(warnings_count == 0),
            violations: vec![],
            total_constraints: 0,
            satisfied_constraints: 0,
            constraints_summary: vec![],
            skip_reason: None,
        };
    }

    // Phase 2: Model checking — use updated_plan so PatternUngrounded requirements are skipped
    let constraints = translator::translate_all(&updated_plan);
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();

    if formalizable.is_empty() {
        // In single-file/stdin mode, having no formalizable constraints is OK (no requirements is expected)
        // In OpenSpec mode, this would have been caught earlier as a blocker
        return VerificationResult {
            plan_name: plan_name.to_string(),
            phase: "model_check".into(),
            convertible: true,
            convertibility_report: Some(conv_report),
            valid: Some(true), // No constraints to check = valid by default in single-file mode
            violations: vec![],
            total_constraints: 0,
            satisfied_constraints: 0,
            constraints_summary: vec![],
            skip_reason: if is_openspec {
                Some("No formalizable constraints to check".into())
            } else {
                None // Single-file mode: no requirements is expected
            },
        };
    }

    if backend == CheckerBackend::Spin
        && let Err(msg) = require_spin()
    {
        // Missing SPIN: plan is convertible, but we can't model-check.
        // In pre-commit mode, this is non-blocking (exit 0 with a warning).
        // In normal mode, this is a hard failure (exit 2) because verification
        // is incomplete.
        return VerificationResult {
            plan_name: plan_name.to_string(),
            phase: "model_check".into(),
            convertible: pre_commit, // true in pre-commit, false in normal mode
            convertibility_report: Some(conv_report),
            valid: None, // Unknown — can't prove without SPIN
            violations: vec![],
            total_constraints: formalizable.len(),
            satisfied_constraints: 0,
            constraints_summary: vec![],
            skip_reason: Some(msg),
        };
    }

    match backend {
        CheckerBackend::Spin => spin::run_spin_check(plan, plan_name, &constraints, conv_report),
        CheckerBackend::SpinRs => spin_rs::run_spin_rs_check(plan, plan_name, &constraints, conv_report),
    }
}

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

/// Verify a plan with a strictness profile.
/// For now, this delegates to `verify()` with the existing behavior.
/// Strictness-based severity mapping will be added in Phase 2.
pub fn verify_with_strictness(
    plan: &PlanIR,
    plan_name: &str,
    no_model: bool,
    pre_commit: bool,
    strictness: crate::input::StrictnessProfile,
    is_openspec: bool,
    backend: CheckerBackend,
) -> VerificationResult {
    let mut result = verify(plan, plan_name, no_model, pre_commit, is_openspec, backend);

    // Apply strictness-based severity mapping
    result = apply_strictness(result, strictness, is_openspec);

    result
}

/// Apply strictness profile to adjust severity of check items.
fn apply_strictness(
    mut result: VerificationResult,
    strictness: crate::input::StrictnessProfile,
    _is_openspec: bool,
) -> VerificationResult {
    if let Some(ref mut report) = result.convertibility_report {
        for item in report.blockers.iter_mut() {
            item.severity = strictness_severity(&item.check, strictness).to_string();
        }

        let mut new_blockers = Vec::new();
        let mut new_warnings = Vec::new();
        let mut new_info = Vec::new();

        drain_by_severity(&mut report.blockers, &mut new_blockers, &mut new_warnings, &mut new_info);
        drain_by_severity(&mut report.warnings, &mut new_blockers, &mut new_warnings, &mut new_info);
        new_info.append(&mut report.info);

        report.blockers = new_blockers;
        report.warnings = new_warnings;
        report.info = new_info;

        report.status = if !report.blockers.is_empty() {
            crate::ir::ConvertibilityStatus::Blocking
        } else if !report.warnings.is_empty() {
            crate::ir::ConvertibilityStatus::ConvertibleWithWarnings
        } else {
            crate::ir::ConvertibilityStatus::Convertible
        };

        let was_blocking = !result.convertible;
        let now_blocking = report.status == crate::ir::ConvertibilityStatus::Blocking;
        result.convertible = !now_blocking;

        if now_blocking && !was_blocking {
            result.skip_reason = Some("Convertibility check failed".into());
            result.valid = None;
        } else if !now_blocking && was_blocking {
            result.skip_reason = None;
            result.valid = None;
        }
    }
    result
}

fn strictness_severity(check: &str, strictness: crate::input::StrictnessProfile) -> &'static str {
    match strictness {
        crate::input::StrictnessProfile::Strict => "blocker",
        crate::input::StrictnessProfile::Moderate => {
            if check == "pattern_ungrounded"
                || check == "no_requirements"
                || check == "no_tasks"
            {
                "warning"
            } else {
                "blocker"
            }
        }
        crate::input::StrictnessProfile::Lax => {
            if check == "pattern_ungrounded"
                || check == "no_requirements"
                || check == "no_tasks"
            {
                "info"
            } else {
                "blocker"
            }
        }
    }
}

fn drain_by_severity(
    source: &mut Vec<crate::ir::CheckItem>,
    blockers: &mut Vec<crate::ir::CheckItem>,
    warnings: &mut Vec<crate::ir::CheckItem>,
    info: &mut Vec<crate::ir::CheckItem>,
) {
    for item in source.drain(..) {
        match item.severity.as_str() {
            "blocker" => blockers.push(item),
            "warning" => warnings.push(item),
            _ => info.push(item),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::CheckItem;

    fn make_result(plan_name: &str, convertible: bool, valid: Option<bool>, violations: usize) -> VerificationResult {
        VerificationResult {
            plan_name: plan_name.into(),
            phase: "full".into(),
            convertible,
            convertibility_report: None,
            valid,
            violations: (0..violations).map(|i| Violation {
                constraint_id: format!("R{}", i),
                requirement_statement: "test".into(),
                ltl: String::new(),
                category: String::new(),
                state: String::new(),
                task_source: None,
                req_source: None,
                suggested_fix: None,
                plan: plan_name.into(),
            }).collect(),
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

    #[test]
    fn test_drain_by_severity() {
        let mut items = vec![
            CheckItem {
                check: "test".into(),
                element: "e1".into(),
                location: "l1".into(),
                detail: "d1".into(),
                severity: "blocker".into(),
                fix: None,
            },
            CheckItem {
                check: "test".into(),
                element: "e2".into(),
                location: "l2".into(),
                detail: "d2".into(),
                severity: "warning".into(),
                fix: None,
            },
            CheckItem {
                check: "test".into(),
                element: "e3".into(),
                location: "l3".into(),
                detail: "d3".into(),
                severity: "info".into(),
                fix: None,
            },
        ];
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();
        let mut info = Vec::new();
        drain_by_severity(&mut items, &mut blockers, &mut warnings, &mut info);
        assert_eq!(blockers.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert_eq!(info.len(), 1);
        assert!(items.is_empty());
    }
}

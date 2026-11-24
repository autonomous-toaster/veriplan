//! Verifier engine: convertibility check → Promela generation → model checking.
//!
//! Three phases:
//!   1. Convertibility check (Phase 1): validate plan can become a formal model
//!   2. Promela + SPIN (Phase 2a): full SPIN model checking
//!   3. BFS fallback (Phase 2b): built-in explorer when SPIN unavailable
#![allow(dead_code)]

pub mod bfs;
pub(crate) mod canary;
pub(crate) mod checks;
mod convertibility;
#[cfg(test)]
mod engine_gate;
pub(crate) mod merge;
pub mod promela;
pub mod spin;
pub mod spin_rs;

pub use convertibility::check_convertibility;
pub use merge::{merge_results, verify_all};

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
    /// The curated problem kind (violation_*), stable across strictness.
    pub kind: crate::ir::Kind,
    /// The remedy this violation prescribes (drives `--fix`).
    pub op: crate::ir::Op,
}

/// Summary of one checked constraint with pass/fail status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstraintSummary {
    pub requirement_id: String,
    pub statement: String,
    pub category: String,
    pub satisfied: bool,
    pub unchecked: bool,
    /// False when the constraint carried no formula and was therefore never checked.
    pub modelled: bool,
    /// Why no verdict was produced, when the engine reported a reason: one of
    /// unsupported_property, unparsable_formula, state_budget, depth_budget, early_stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchecked_reason: Option<String>,
    /// States the claim's traversal stored, when the engine reports it.
    ///
    /// A caller deciding whether to raise the budget needs the ratio, not a sentence: "used up"
    /// does not say whether the claim came four thousand states short or four million.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub states_used: Option<usize>,
    /// States the claim was allowed, when the engine reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub states_allowed: Option<usize>,
    /// The requirement body is grounded on an archived change, so no formula describes this plan.
    pub historical: bool,
    /// The archived change that owns the body, when `historical` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historical_owner: Option<String>,
    /// Why the constraint was not modelled, when that is the case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmodelled_reason: Option<String>,
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
    /// Constraints that carried no formula. Not counted as satisfied.
    pub unmodelled_constraints: usize,
    /// Constraints whose search did not complete within the budget.
    pub unchecked_constraints: usize,
    /// Constraints whose body is grounded on history. Not checkable, and never counted as satisfied.
    pub historical_constraints: usize,
    /// The backend that produced the verdict. Named so two runs cannot be confused.
    pub backend: String,
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

/// Human name of a backend, used in the verdict so two runs cannot be confused.
fn backend_label(backend: CheckerBackend) -> &'static str {
    match backend {
        CheckerBackend::Spin => "spin",
        CheckerBackend::SpinRs => "spin-rs",
    }
}

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
            unmodelled_constraints: 0,
            unchecked_constraints: 0,
            historical_constraints: 0,
            backend: backend_label(backend).to_string(),
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
            unmodelled_constraints: 0,
            unchecked_constraints: 0,
            historical_constraints: 0,
            backend: backend_label(backend).to_string(),
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
        // Placeholders are gone, so a plan can hold constraints of which none is modelled.
        // That is not a pass: nothing was checked.
        let unmodelled = constraints
            .iter()
            .filter(|c| !c.modelled() && !c.historical())
            .count();
        let historical = constraints.iter().filter(|c| c.historical()).count();
        let (valid, skip_reason) = if constraints.is_empty() {
            (Some(true), None) // nothing to check; single-file input is expected to be empty
        } else {
            (
                None,
                Some(format!(
                    "no constraint could be modelled ({} constraint(s): {} unmodelled, {} historical)",
                    constraints.len(),
                    unmodelled,
                    historical
                )),
            )
        };
        return VerificationResult {
            plan_name: plan_name.to_string(),
            phase: "model_check".into(),
            convertible: true,
            convertibility_report: Some(conv_report),
            valid,
            violations: vec![],
            total_constraints: constraints.len(),
            satisfied_constraints: 0,
            unmodelled_constraints: unmodelled,
            unchecked_constraints: 0,
            historical_constraints: historical,
            backend: backend_label(backend).to_string(),
            constraints_summary: vec![],
            skip_reason,
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
            total_constraints: constraints.len(),
            satisfied_constraints: 0,
            unmodelled_constraints: constraints.iter().filter(|c| !c.modelled()).count(),
            unchecked_constraints: formalizable.len(),
            historical_constraints: 0,
            backend: backend_label(backend).to_string(),
            constraints_summary: vec![],
            skip_reason: Some(msg),
        };
    }

    // Calibration: a backend that cannot refute a model with a known violation cannot be
    // trusted to judge this one. Unverified, never valid and never invalid.
    match canary::calibrated(backend) {
        Ok(true) => {}
        Ok(false) => {
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
                unchecked_constraints: formalizable.len(),
                historical_constraints: 0,
                backend: backend_label(backend).to_string(),
                constraints_summary: vec![],
                skip_reason: Some(
                    "the backend did not refute the calibration canary: a model with a known \
                     violation was reported clean, so its verdict on this plan means nothing"
                        .into(),
                ),
            };
        }
        Err(msg) => {
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
                unchecked_constraints: formalizable.len(),
                historical_constraints: 0,
                backend: backend_label(backend).to_string(),
                constraints_summary: vec![],
                skip_reason: Some(format!("calibration canary could not run: {msg}")),
            };
        }
    }

    match backend {
        CheckerBackend::Spin => spin::run_spin_check(plan, plan_name, &constraints, conv_report),
        CheckerBackend::SpinRs => {
            spin_rs::run_spin_rs_check(plan, plan_name, &constraints, conv_report)
        }
    }
}

/// Verify a plan with a strictness profile.
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

    // Add prose-guidance (curated steve rules) as advisory findings and
    // correlate with grounding. Prose findings NEVER block — they only add
    // warnings/info and rephrase directives.
    if let Some(report) = result.convertibility_report.as_mut() {
        let findings = crate::prose::check_prose(plan, None, &strictness);
        for f in &findings {
            // Carry steve's structured fields (fixability/replacement/ste_rule)
            // onto the emitted CheckItem and tag prose findings advisory
            // (design D7 / task 4.4). Prose findings never block.
            let item = crate::ir::CheckItem {
                excerpt: Some(f.snippet.clone()),
                severity: f.severity.clone(),
                check: f.rule.clone(),
                element: f.element.clone(),
                location: format!("{}:{}", f.file, f.line),
                detail: f.message.clone(),
                fix: f.suggestion.clone(),
                kind: crate::ir::kind_of(&f.rule),
                op: if f.fixability == crate::ir::Fixability::Local {
                    crate::ir::Op::ReplaceBody
                } else {
                    crate::ir::Op::None
                },
                fixability: f.fixability,
                start: f.start,
                end: f.end,
                replacement: f.replacement.clone(),
            };
            // Route by severity so a `blocker` prose finding reaches
            // `report.blockers` (it used to fall through to `report.info`),
            // giving it a visible severity that the verdict and exit code scan.
            match item.severity.as_str() {
                "blocker" => report.blockers.push(item),
                "warning" => report.warnings.push(item),
                _ => report.info.push(item),
            }
            if let Some(s) = &f.suggestion {
                report.rephrase_directives.push(format!(
                    "[{}] {}: {}",
                    f.severity.to_uppercase(),
                    f.element,
                    s
                ));
            }
        }
        // Correlation directives (fix style AND grounding in one step).
        for d in crate::prose::correlate_with_grounding(&findings, plan, &strictness) {
            report.rephrase_directives.push(format!(
                "[{}] {}",
                d.severity.to_uppercase(),
                d.directive
            ));
        }
    }

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

        drain_by_severity(
            &mut report.blockers,
            &mut new_blockers,
            &mut new_warnings,
            &mut new_info,
        );
        drain_by_severity(
            &mut report.warnings,
            &mut new_blockers,
            &mut new_warnings,
            &mut new_info,
        );
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
            if check == "pattern_ungrounded" || check == "no_requirements" || check == "no_tasks" {
                "warning"
            } else {
                "blocker"
            }
        }
        crate::input::StrictnessProfile::Lax => {
            if check == "pattern_ungrounded" || check == "no_requirements" || check == "no_tasks" {
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
mod tests;

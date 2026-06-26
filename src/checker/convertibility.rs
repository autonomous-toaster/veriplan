//! Convertibility check orchestration — coordinates individual checks.

use crate::checker::checks;

use crate::ir::{ConvertibilityReport, ConvertibilityStatus, PlanIR};

/// Run the full convertibility check (Phase 1).
pub fn check_convertibility(plan: &PlanIR, is_openspec: bool) -> ConvertibilityReport {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    // Check 1: Tasks exist and have unique IDs
    let task_check = checks::check_tasks(plan, is_openspec);
    if let Some(b) = task_check.0 {
        blockers.push(b)
    }
    warnings.extend(task_check.1);
    info.extend(task_check.2);

    // Check 2: Requirements exist and have RFC 2119 keywords
    let req_check = checks::check_requirements(plan, is_openspec);
    blockers.extend(req_check.0);
    warnings.extend(req_check.1);
    info.extend(req_check.2);

    // Check 3: Task references
    let ref_check = checks::check_task_references(plan);
    blockers.extend(ref_check.0);
    warnings.extend(ref_check.1);

    // Check 4: Temporal classifiability
    let class_check = checks::check_classifiability(plan, is_openspec);
    blockers.extend(class_check.0);
    warnings.extend(class_check.1);
    info.extend(class_check.2);

    // Check 5: Scenario completeness
    let sc_check = checks::check_scenarios(plan);
    warnings.extend(sc_check.0);
    info.extend(sc_check.1);

    // Check 6: Constraint diversity
    info.extend(checks::check_diversity(plan));

    // Check 7: Task coverage
    let cov_check = checks::check_task_coverage(plan, is_openspec);
    warnings.extend(cov_check.0);

    // Build rephrase directives
    let mut rephrase_directives = Vec::new();
    for b in &blockers {
        if let Some(fix) = &b.fix {
            rephrase_directives.push(format!("[BLOCKER] {}: {}", b.element, fix));
        }
    }
    for w in &warnings {
        if let Some(fix) = &w.fix {
            rephrase_directives.push(format!("[WARNING] {}: {}", w.element, fix));
        }
    }

    let status = if !blockers.is_empty() {
        ConvertibilityStatus::Blocking
    } else if !warnings.is_empty() {
        ConvertibilityStatus::ConvertibleWithWarnings
    } else {
        ConvertibilityStatus::Convertible
    };

    ConvertibilityReport {
        status,
        blockers,
        warnings,
        info,
        rephrase_directives,
    }
}

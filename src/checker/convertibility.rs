//! Convertibility check orchestration — coordinates individual checks.

use crate::checker::checks;
use crate::grounding;

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

    // Identify requirement IDs that failed T4.2 (bad task references)
    // so we can skip grounding for them (no point grounding if task IDs don't exist)
    let failed_ref_ids: std::collections::HashSet<String> = blockers
        .iter()
        .filter(|b| b.check == "bad_task_reference")
        .map(|b| {
            // Extract requirement ID from element string "Requirement 'R1'"
            b.element
                .strip_prefix("Requirement '")
                .and_then(|s| s.strip_suffix('\''))
                .unwrap_or("")
                .to_string()
        })
        .collect();

    // Check 4: Grounding check (between T4.2 and T4.4)
    // Skip grounding for requirements that already failed T4.2
    let (g_blockers, g_warnings, g_info, g_outcomes) = if failed_ref_ids.is_empty() {
        grounding::check_grounding(plan, &crate::input::StrictnessProfile::Strict)
    } else {
        // Create a filtered plan excluding requirements with bad references
        let mut filtered = plan.clone();
        filtered.requirements.retain(|r| !failed_ref_ids.contains(&r.id));
        grounding::check_grounding(&filtered, &crate::input::StrictnessProfile::Strict)
    };
    blockers.extend(g_blockers);
    warnings.extend(g_warnings);
    info.extend(g_info);

    // Populate PatternUngrounded on requirements that failed grounding
    let mut updated_plan = plan.clone();
    for outcome in &g_outcomes {
        if outcome.failed
            && let Some(req) = updated_plan
                .requirements
                .iter_mut()
                .find(|r| r.id == outcome.requirement_id)
            {
                req.category = crate::ir::ConstraintCategory::PatternUngrounded;
            }
    }
    // Note: updated_plan is not used further here since the report is already built.
    // The category update is for downstream consumers that read PlanIR after convertibility.
    // For now, the grounding CheckItems in the report carry the information.
    let _ = updated_plan;

    // Check 5: Temporal classifiability
    let class_check = checks::check_classifiability(plan, is_openspec);
    blockers.extend(class_check.0);
    warnings.extend(class_check.1);
    info.extend(class_check.2);

    // Check 6: Scenario completeness
    let sc_check = checks::check_scenarios(plan);
    warnings.extend(sc_check.0);
    info.extend(sc_check.1);

    // Check 7: Constraint diversity
    info.extend(checks::check_diversity(plan));

    // Check 8: Task coverage
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

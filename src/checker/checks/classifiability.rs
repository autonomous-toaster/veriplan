//! Classification check (non-formalizable / pattern-ungrounded diagnosis) —
//! extracted from checks.rs to keep files under the 550-line file-length gate.

use crate::ir::{CheckItem, ConstraintCategory, PlanIR};
use crate::translator;
use crate::util::truncate;

use super::make_item;

/// Classify every requirement into a formal model category, blocking
/// requirements that cannot be model-checked and explaining why.
pub fn check_classifiability(
    plan: &PlanIR,
    _is_openspec: bool,
) -> (Vec<CheckItem>, Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let warnings = Vec::new();
    let mut info = Vec::new();

    let task_ids: Vec<String> = plan.tasks.iter().map(|t| t.id.clone()).collect();
    let mut formalizable_count = 0;
    let mut non_formalizable_count = 0;

    for req in &plan.requirements {
        if req.strength == crate::ir::Rfc2119Strength::May {
            info.push(make_item(
                "info",
                "may_requirement",
                format!("Requirement '{}'", req.id),
                format!("{}:{}", req.source.file, req.source.start_line),
                format!(
                    "MAY '{}' is informational — not verified by model checking",
                    truncate(&req.statement, 80)
                ),
                None,
            ));
            continue;
        }
        let cat = translator::classify(&req.statement);

        if cat == ConstraintCategory::Informational {
            // Informational / human-review-only requirement: not a temporal
            // constraint; surface as INFO, do not block and do not count
            // toward non-formalizable.
            info.push(make_item(
                "info",
                "informational_requirement",
                format!("Requirement '{}'", req.id),
                format!("{}:{}", req.source.file, req.source.start_line),
                format!(
                    "Informational '{}' — human review only, not verified by model checking",
                    truncate(&req.statement, 80)
                ),
                None,
            ));
            continue;
        }

        let cat = if cat != ConstraintCategory::NonFormalizable
            && cat != ConstraintCategory::PatternUngrounded
        {
            let refs = translator::extract_task_refs_bare(&req.statement, &task_ids);
            if refs.is_empty() {
                ConstraintCategory::PatternUngrounded
            } else {
                cat
            }
        } else {
            cat
        };

        if cat == ConstraintCategory::NonFormalizable {
            non_formalizable_count += 1;
            // Diagnose WHY it is non-formalizable to emit a targeted fix.
            let diagnosis = translator::diagnose_vague(&req.statement, &task_ids);
            let (detail, fix, check) = match &diagnosis {
                Some(translator::VagueDiagnosis::BareCapability { task, .. }) => (
                    format!(
                        "SHALL '{}' references task {} but specifies no constraint — this is redundant with the task list",
                        truncate(&req.statement, 80),
                        task
                    ),
                    format!(
                        "add a temporal relation to another task (e.g. '{} SHALL complete BEFORE T1.2 SHALL start'), or remove it if it merely re-states the task",
                        task
                    ),
                    "bare_capability",
                ),
                Some(translator::VagueDiagnosis::VagueAction { task, word }) => (
                    format!(
                        "SHALL '{}' references task {} but '{}' is vague and not objectively testable",
                        truncate(&req.statement, 80),
                        task,
                        word
                    ),
                    format!(
                        "define it measurably (e.g. 'within 200ms'), or add a temporal relation to another task (e.g. '{} SHALL complete BEFORE T1.2 SHALL start')",
                        task
                    ),
                    "vague_action",
                ),
                Some(translator::VagueDiagnosis::VagueQuality { word }) => (
                    format!(
                        "SHALL '{}' has no task reference and '{}' is vague",
                        truncate(&req.statement, 80),
                        word
                    ),
                    format!(
                        "reference a task with a temporal relation, or define '{}' via a measurable criterion or standard (e.g. express a safety statement as 'T1.1 SHALL fail safe IF T1.2 SHALL fail')",
                        word
                    ),
                    "vague_quality",
                ),
                None => (
                    format!(
                        "SHALL '{}' does not match any temporal category",
                        truncate(&req.statement, 80)
                    ),
                    "Rewrite as: sequential, exclusive, conditional, concurrent, or global constraint"
                        .to_string(),
                    "unknown_non_formalizable",
                ),
            };
            blockers.push(make_item(
                "blocker",
                check,
                format!("Requirement '{}'", req.id),
                format!("{}:{}", req.source.file, req.source.start_line),
                detail,
                Some(fix),
            ));
        } else if cat == ConstraintCategory::PatternUngrounded {
            formalizable_count += 1;
            blockers.push(make_item(
                "blocker",
                "pattern_ungrounded",
                format!("Requirement '{}'", req.id),
                format!("{}:{}", req.source.file, req.source.start_line),
                format!(
                    "SHALL '{}' has a temporal pattern but no task references — add task IDs for model verification",
                    truncate(&req.statement, 80)
                ),
                Some(
                    "Add task ID references (e.g., T1.2) to enable model verification".into(),
                ),
            ));
        } else {
            formalizable_count += 1;
        }
    }

    if formalizable_count == 0 && non_formalizable_count > 0 {
        blockers.push(make_item(
            "blocker",
            "no_formalizable",
            "Plan".into(),
            "specs/".into(),
            "No requirements are classifiable into a temporal category".into(),
            Some("Rewrite all requirements using temporal constraint patterns".into()),
        ));
    }

    info.push(make_item(
        "info",
        "classification_summary",
        "Plan".into(),
        "specs/".into(),
        format!(
            "{} formalizable, {} non-formalizable requirements",
            formalizable_count, non_formalizable_count
        ),
        None,
    ));

    (blockers, warnings, info)
}

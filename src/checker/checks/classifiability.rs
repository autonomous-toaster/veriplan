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

        // A modelled constraint is built from tasks, so it needs task ids: one to name a task, and
        // two for the pair categories (the pair kept apart, or the trigger and the consequent). A body
        // that cites fewer yields no formula, and is reported here rather than passing silently.
        let classified = cat.clone();
        let needed = match classified {
            ConstraintCategory::Exclusive
            | ConstraintCategory::Conditional
            | ConstraintCategory::ConcurrentEvents => 2,
            ConstraintCategory::NonFormalizable | ConstraintCategory::PatternUngrounded => 0,
            _ => 1,
        };
        let cat = if needed == 0 {
            classified.clone()
        } else {
            let available = ids_for(req, &task_ids);
            let refs = translator::extract_task_refs_bare(&req.statement, &available);
            if refs.len() < needed {
                ConstraintCategory::PatternUngrounded
            } else {
                classified.clone()
            }
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
            let (detail, fix) = ungrounded_argument(classified, &req.statement);
            blockers.push(make_item(
                "blocker",
                "pattern_ungrounded",
                format!("Requirement '{}'", req.id),
                format!("{}:{}", req.source.file, req.source.start_line),
                detail,
                Some(fix),
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

    (blockers, warnings, info)
}

/// Why a body with a temporal keyword is not modelled, and what would make it modelable.
///
/// The three non-sequential categories are built from a pair of tasks, so a body that cites fewer
/// than two task ids yields no formula. "Add task IDs" leaves the author to guess which two the
/// keyword needs; naming the argument is the whole remedy.
fn ungrounded_argument(classified: ConstraintCategory, statement: &str) -> (String, String) {
    let quoted = format!("SHALL '{}'", truncate(statement, 80));
    match classified {
        ConstraintCategory::Conditional => (
            format!(
                "{quoted} is conditional, so this body needs two task ids — a trigger task and a consequent task — and cites fewer"
            ),
            "cite the trigger and the consequent, e.g. 'IF T1.1 fails THEN T1.2 SHALL retry'".into(),
        ),
        ConstraintCategory::Exclusive => (
            format!(
                "{quoted} is exclusive, so this body needs two task ids — the pair the constraint keeps apart — and cites fewer"
            ),
            "cite the pair, e.g. 'AT MOST ONE of T1.1 and T1.2 SHALL be active'".into(),
        ),
        ConstraintCategory::ConcurrentEvents => (
            format!(
                "{quoted} is concurrent, so this body needs two task ids in one concurrent phase, and cites fewer"
            ),
            "cite two tasks of one [concurrent] phase, e.g. 'T1.1 SHALL run CONCURRENTLY with T1.2'"
                .into(),
        ),
        _ => (
            format!("{quoted} has a temporal pattern but no task references — add task IDs for model verification"),
            "Add task ID references (e.g., T1.2) to enable model verification".into(),
        ),
    }
}

/// The task ids a requirement may cite: this change's tasks, plus the tasks of the change that
/// wrote the requirement's body when a provenance was resolved.
fn ids_for(req: &crate::ir::Requirement, plan_ids: &[String]) -> Vec<String> {
    let mut ids = plan_ids.to_vec();
    if let Some(history) = &req.grounded_in {
        for id in &history.tasks {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
    }
    ids
}

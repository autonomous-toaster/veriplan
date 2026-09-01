//! Individual convertibility checks — extracted from convertibility.rs.

use std::collections::HashMap;

use crate::ir::{
    CheckItem, ConstraintCategory, Fixability, Op, PlanIR, Rfc2119Strength, StepKind, Task,
};
use crate::translator;

/// Build a `CheckItem` with `kind`/`op`/`fixability` derived from the check
/// value (design D2/D3). `op` and `fixability` default conservatively and are
/// refined per-check where a deterministic edit exists.
pub(crate) fn make_item(
    severity: &str,
    check: &str,
    element: String,
    location: String,
    detail: String,
    fix: Option<String>,
) -> CheckItem {
    let kind = crate::ir::kind_of(check);
    // Default: no remedy, needs judgment. Refined below for machine-applicable
    // checks (only `duplicate_task_id` is `Local` per design D3).
    let (op, fixability) = match check {
        "duplicate_task_id" => (Op::RenameTask, Fixability::Local),
        "bad_task_reference" => (Op::FixReference, Fixability::Structural),
        "no_tasks" => (Op::AddTaskReference, Fixability::Structural),
        "no_requirements" => (Op::ReplaceBody, Fixability::Structural),
        "no_phase_grouping" => (Op::AddTaskReference, Fixability::Structural),
        "no_rfc2119_keyword" | "no_rfc2119_any" => (Op::AddTemporalKeyword, Fixability::Structural),
        "bare_capability" | "vague_action" | "vague_quality" | "unknown_non_formalizable" => {
            (Op::ReplaceBody, Fixability::Structural)
        }
        "pattern_ungrounded" => (Op::AddTaskReference, Fixability::Structural),
        "no_formalizable" => (Op::ReplaceBody, Fixability::Structural),
        "grounding_ambiguous_multi_keyword" | "grounding_multi_keyword" => {
            (Op::SplitRequirement, Fixability::Structural)
        }
        "grounding_ambiguous" | "grounding_ungroundable" => {
            (Op::AddTemporalKeyword, Fixability::Structural)
        }
        "scenario_no_when" | "scenario_no_then" => (Op::AddScenarioStep, Fixability::Structural),
        "then_no_shall" => (Op::AddTemporalKeyword, Fixability::Structural),
        "task_not_covered" => (Op::AddTaskReference, Fixability::Structural),
        "may_requirement" | "informational_requirement" => {
            (Op::InformationalOnly, Fixability::Structural)
        }
        _ => (Op::None, Fixability::Structural),
    };
    CheckItem {
        severity: severity.into(),
        check: check.into(),
        element,
        location,
        detail,
        fix,
        kind,
        op,
        fixability,
        start: 0,
        end: 0,
        replacement: None,
    }
}

#[cfg(test)]
mod checks_tests;

/// Check 1: Tasks exist and have unique IDs.
pub fn check_tasks(
    plan: &PlanIR,
    is_openspec: bool,
) -> (Option<CheckItem>, Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    if plan.tasks.is_empty() {
        let severity = if is_openspec { "blocker" } else { "info" };
        let item = make_item(
            severity,
            "no_tasks",
            "Plan".into(),
            "tasks.md".into(),
            "No tasks found in plan".into(),
            Some("Add at least one task with N.M ID to tasks.md".into()),
        );
        if severity == "blocker" {
            blockers.push(item);
            return (blockers.pop(), Vec::new(), Vec::new());
        } else {
            return (None, Vec::new(), vec![item]);
        }
    }

    let mut seen_ids: HashMap<&str, &Task> = HashMap::new();
    for task in &plan.tasks {
        if let Some(existing) = seen_ids.get(task.id.as_str()) {
            blockers.push(make_item(
                "blocker",
                "duplicate_task_id",
                format!("Task {}", task.id),
                format!("{}:{}", task.source.file, task.source.start_line),
                format!(
                    "Duplicate task ID '{}' also at {}:{}",
                    task.id, existing.source.file, existing.source.start_line
                ),
                Some(format!("Rename one of the tasks with ID '{}'", task.id)),
            ));
        } else {
            seen_ids.insert(&task.id, task);
        }
    }

    // Check for isolated tasks (no ordering context)
    if plan.phases.is_empty() && plan.tasks.len() > 1 {
        warnings.push(make_item(
            "warning",
            "no_phase_grouping",
            "Plan".into(),
            "tasks.md".into(),
            "No phase groupings found — tasks may lack ordering context".into(),
            Some("Add ## Phase section headings to group tasks".into()),
        ));
    }

    info.push(make_item(
        "info",
        "task_count",
        "Plan".into(),
        "tasks.md".into(),
        format!(
            "Found {} tasks across {} phases",
            plan.tasks.len(),
            plan.phases.len()
        ),
        None,
    ));

    (None, warnings, info)
}

/// Check 2: Requirements exist and have RFC 2119 keywords.
pub fn check_requirements(
    plan: &PlanIR,
    is_openspec: bool,
) -> (Vec<CheckItem>, Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if plan.requirements.is_empty() {
        let severity = if is_openspec { "blocker" } else { "info" };
        let item = make_item(
            severity,
            "no_requirements",
            "Plan".into(),
            "specs/".into(),
            "No requirements found in any spec file".into(),
            Some("Add ### Requirement: sections with SHALL/MUST paragraphs to spec files".into()),
        );
        if severity == "blocker" {
            blockers.push(item);
        } else {
            return (blockers, warnings, vec![item]);
        }
        return (blockers, warnings, Vec::new());
    }

    let mut has_rfc2119 = false;
    for req in &plan.requirements {
        if req.strength == Rfc2119Strength::None {
            warnings.push(make_item(
                "warning",
                "no_rfc2119_keyword",
                format!("Requirement '{}'", req.id),
                format!("{}:{}", req.source.file, req.source.start_line),
                format!("No RFC 2119 keyword found: '{}'", req.statement),
                Some(
                    "Use SHALL/MUST (hard), SHOULD (soft), or MAY (optional) in the requirement"
                        .into(),
                ),
            ));
        } else {
            has_rfc2119 = true;
        }
    }

    if !has_rfc2119 {
        warnings.push(make_item(
            "warning",
            "no_rfc2119_any",
            "Plan".into(),
            "specs/".into(),
            "No requirements use RFC 2119 keywords (SHALL/MUST/SHOULD/MAY)".into(),
            Some("Add SHALL/MUST/SHOULD/MAY constraints to make requirements verifiable".into()),
        ));
    }

    (blockers, warnings, Vec::new())
}

/// Check 3: Task references in requirements.
pub fn check_task_references(plan: &PlanIR) -> (Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let warnings = Vec::new();

    let task_ids: Vec<String> = plan.tasks.iter().map(|t| t.id.clone()).collect();

    for req in &plan.requirements {
        let refs = translator::extract_task_refs_bare(&req.statement, &task_ids);
        for ref_id in refs {
            if !task_ids.contains(&ref_id) {
                blockers.push(make_item(
                    "blocker",
                    "bad_task_reference",
                    format!("Requirement '{}'", req.id),
                    format!("{}:{}", req.source.file, req.source.start_line),
                    format!("References task '{}' but no such task exists", ref_id),
                    Some(format!(
                        "Change '{}' to a valid task ID: {:?}",
                        ref_id,
                        task_ids.iter().take(5).collect::<Vec<_>>()
                    )),
                ));
            }
        }
    }

    (blockers, warnings)
}

mod classifiability;
pub use classifiability::check_classifiability;
pub fn check_scenarios(plan: &PlanIR) -> (Vec<CheckItem>, Vec<CheckItem>) {
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    for sc in &plan.scenarios {
        let has_when = sc.steps.iter().any(|s| s.kind == StepKind::When);
        let has_then = sc.steps.iter().any(|s| s.kind == StepKind::Then);

        if !has_when {
            warnings.push(make_scenario_warning(
                &sc.name,
                &sc.source,
                "scenario_no_when",
                "Scenario missing WHEN step",
                "Add '- **WHEN** ...' to the scenario",
            ));
        }
        if !has_then {
            warnings.push(make_scenario_warning(
                &sc.name,
                &sc.source,
                "scenario_no_then",
                "Scenario missing THEN step",
                "Add '- **THEN** ... SHALL ...' to the scenario",
            ));
        }

        if has_then {
            warnings.extend(check_then_steps_rfc2119(sc));
        }
    }

    info.push(make_item(
        "info",
        "scenario_count",
        "Plan".into(),
        "specs/".into(),
        format!(
            "Found {} scenarios across all spec files",
            plan.scenarios.len()
        ),
        None,
    ));

    (warnings, info)
}

/// Create a warning CheckItem for a scenario issue.
fn make_scenario_warning(
    name: &str,
    source: &crate::ir::SourceLocation,
    check: &str,
    detail: &str,
    fix: &str,
) -> CheckItem {
    make_item(
        "warning",
        check,
        format!("Scenario '{}'", name),
        format!("{}:{}", source.file, source.start_line),
        detail.into(),
        Some(fix.into()),
    )
}

/// Check THEN/AND steps for RFC 2119 keywords.
fn check_then_steps_rfc2119(sc: &crate::ir::Scenario) -> Vec<CheckItem> {
    let mut warnings = Vec::new();
    for step in &sc.steps {
        if step.kind == StepKind::Then || step.kind == StepKind::And {
            let strength = crate::parser::detect_rfc2119(&step.text);
            if strength == Rfc2119Strength::None {
                warnings.push(make_item(
                    "warning",
                    "then_no_shall",
                    format!("Scenario '{}'", sc.name),
                    format!("{}:{}", sc.source.file, step.source.start_line),
                    format!("{:?} step has no RFC 2119 keyword", step.kind),
                    Some("Add SHALL/MUST/SHOULD to the step".into()),
                ));
            }
        }
    }
    warnings
}

/// Check 6: Constraint diversity.
pub fn check_diversity(plan: &PlanIR) -> Vec<CheckItem> {
    let mut cat_counts: HashMap<&str, usize> = HashMap::new();
    for req in &plan.requirements {
        let label = match translator::classify(&req.statement) {
            ConstraintCategory::FixedTime => "fixed_time",
            ConstraintCategory::SequentialOrder => "sequential",
            ConstraintCategory::ConcurrentEvents => "concurrent",
            ConstraintCategory::Conditional => "conditional",
            ConstraintCategory::Exclusive => "exclusive",
            ConstraintCategory::Global => "global",
            ConstraintCategory::NonFormalizable => "non_formalizable",
            ConstraintCategory::PatternUngrounded => "pattern_ungrounded",
            ConstraintCategory::Informational => continue,
        };
        *cat_counts.entry(label).or_insert(0) += 1;
    }

    let mut summary: Vec<String> = cat_counts
        .iter()
        .map(|(k, v)| format!("{}({})", k, v))
        .collect();
    summary.sort();

    let total: usize = cat_counts.values().sum();
    if total == 0 {
        return vec![];
    }

    let mut info = Vec::new();
    let formalizable_count = cat_counts
        .iter()
        .filter(|(k, _)| **k != "non_formalizable")
        .map(|(_, v)| v)
        .sum::<usize>();

    let categories_used = cat_counts
        .iter()
        .filter(|(k, _)| **k != "non_formalizable")
        .count();

    if categories_used <= 1 && formalizable_count >= 3 {
        info.push(make_item(
            "info",
            "low_diversity",
            "Plan".into(),
            "specs/".into(),
            format!(
                "Constraint distribution: {}. Consider adding other constraint types for stronger verification",
                summary.join(", ")
            ),
            Some("Add exclusive (mutex), conditional (if-then), or concurrent constraints".into()),
        ));
    }

    info.push(make_item(
        "info",
        "constraint_diversity",
        "Plan".into(),
        "specs/".into(),
        format!("Constraint distribution: {}", summary.join(", ")),
        None,
    ));

    info
}

/// Check 7: Task coverage — every task should be referenced by at least one SHALL.
pub fn check_task_coverage(plan: &PlanIR, is_openspec: bool) -> (Vec<CheckItem>, Vec<CheckItem>) {
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    let mut referenced: std::collections::HashSet<String> = std::collections::HashSet::new();
    for req in &plan.requirements {
        for task in &plan.tasks {
            let dot_id = format!("T{}", task.id);
            if req.statement.contains(&dot_id) {
                referenced.insert(task.id.clone());
            }
        }
    }

    let mut uncovered = 0;
    for task in &plan.tasks {
        if !referenced.contains(&task.id) {
            uncovered += 1;
            let severity = if is_openspec { "warning" } else { "info" };
            let item = make_item(
                severity,
                "task_not_covered",
                format!("T{} ({})", task.id, task.description),
                format!("{}:{}", task.source.file, task.source.start_line),
                format!(
                    "Task T{} is not referenced by any SHALL requirement — its behavior is unchecked.",
                    task.id
                ),
                Some(format!(
                    "Add a SHALL in specs/ that references T{} with a temporal keyword.",
                    task.id
                )),
            );
            if severity == "warning" {
                warnings.push(item);
            } else {
                info.push(item);
            }
        }
    }

    info.push(make_item(
        "info",
        "task_coverage",
        "Plan".into(),
        "tasks.md".into(),
        format!(
            "{}/{} tasks are covered by SHALL requirements",
            plan.tasks.len() - uncovered,
            plan.tasks.len()
        ),
        None,
    ));

    (warnings, info)
}

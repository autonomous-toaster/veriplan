//! Convertibility check (Phase 1) — validate plan can become a formal model.

use std::collections::HashMap;

use crate::ir::{
    CheckItem, ConstraintCategory, ConvertibilityReport, ConvertibilityStatus, PlanIR,
    Rfc2119Strength, StepKind, Task,
};
use crate::translator;

/// Run the full convertibility check (Phase 1).
/// Returns a report with blockers, warnings, and info items.
pub fn check_convertibility(plan: &PlanIR, is_openspec: bool) -> ConvertibilityReport {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    // Check 1: Tasks exist and have unique IDs
    let task_check = check_tasks(plan);
    if let Some(b) = task_check.0 {
        blockers.push(b)
    }
    warnings.extend(task_check.1);
    info.extend(task_check.2);

    // Check 2: Requirements exist and have RFC 2119 keywords
    let req_check = check_requirements(plan, is_openspec);
    blockers.extend(req_check.0);
    warnings.extend(req_check.1);
    info.extend(req_check.2);

    // Check 3: Task references
    let ref_check = check_task_references(plan);
    blockers.extend(ref_check.0);
    warnings.extend(ref_check.1);

    // Check 4: Temporal classifiability
    let class_check = check_classifiability(plan, is_openspec);
    blockers.extend(class_check.0);
    warnings.extend(class_check.1);
    info.extend(class_check.2);

    // Check 5: Scenario completeness
    let sc_check = check_scenarios(plan);
    warnings.extend(sc_check.0);
    info.extend(sc_check.1);

    // Check 6: Constraint diversity
    info.extend(check_diversity(plan));

    // Check 7: Task coverage — every task should be referenced by at least one SHALL
    let cov_check = check_task_coverage(plan, is_openspec);
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

fn check_tasks(plan: &PlanIR) -> (Option<CheckItem>, Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    if plan.tasks.is_empty() {
        blockers.push(CheckItem {
            severity: "blocker".into(),
            check: "no_tasks".into(),
            element: "Plan".into(),
            location: "tasks.md".into(),
            detail: "No tasks found in plan".into(),
            fix: Some("Add at least one task with N.M ID to tasks.md".into()),
        });
        return (blockers.pop(), warnings, info);
    }

    let mut seen_ids: HashMap<&str, &Task> = HashMap::new();
    for task in &plan.tasks {
        if let Some(existing) = seen_ids.get(task.id.as_str()) {
            blockers.push(CheckItem {
                severity: "blocker".into(),
                check: "duplicate_task_id".into(),
                element: format!("Task {}", task.id),
                location: format!("{}:{}", task.source.file, task.source.start_line),
                detail: format!(
                    "Duplicate task ID '{}' also at {}:{}",
                    task.id, existing.source.file, existing.source.start_line
                ),
                fix: Some(format!("Rename one of the tasks with ID '{}'", task.id)),
            });
        } else {
            seen_ids.insert(&task.id, task);
        }
    }

    // Check for isolated tasks (no ordering context)
    // Every task except the first in each phase should have a sequence position
    if plan.phases.is_empty() && plan.tasks.len() > 1 {
        warnings.push(CheckItem {
            severity: "warning".into(),
            check: "no_phase_grouping".into(),
            element: "Plan".into(),
            location: "tasks.md".into(),
            detail: "No phase groupings found — tasks may lack ordering context".into(),
            fix: Some("Add ## Phase section headings to group tasks".into()),
        });
    }

    info.push(CheckItem {
        severity: "info".into(),
        check: "task_count".into(),
        element: "Plan".into(),
        location: "tasks.md".into(),
        detail: format!(
            "Found {} tasks across {} phases",
            plan.tasks.len(),
            plan.phases.len()
        ),
        fix: None,
    });

    (None, warnings, info)
}

fn check_requirements(
    plan: &PlanIR,
    is_openspec: bool,
) -> (Vec<CheckItem>, Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if plan.requirements.is_empty() {
        // In single-file/stdin mode, no requirements is expected (INFO)
        // In OpenSpec mode, it's a blocker/warning/info depending on strictness
        let severity = if is_openspec { "blocker" } else { "info" };
        let item = CheckItem {
            severity: severity.into(),
            check: "no_requirements".into(),
            element: "Plan".into(),
            location: "specs/".into(),
            detail: "No requirements found in any spec file".into(),
            fix: Some(
                "Add ### Requirement: sections with SHALL/MUST paragraphs to spec files".into(),
            ),
        };
        if severity == "blocker" {
            blockers.push(item);
        } else {
            // Will be handled by apply_strictness for OpenSpec mode
            // For single-file mode, it's always info
            return (blockers, warnings, vec![item]);
        }
        return (blockers, warnings, Vec::new());
    }

    let mut has_rfc2119 = false;
    for req in &plan.requirements {
        if req.strength == Rfc2119Strength::None {
            warnings.push(CheckItem {
                severity: "warning".into(),
                check: "no_rfc2119_keyword".into(),
                element: format!("Requirement '{}'", req.id),
                location: format!("{}:{}", req.source.file, req.source.start_line),
                detail: format!("No RFC 2119 keyword found: '{}'", req.statement),
                fix: Some(
                    "Use SHALL/MUST (hard), SHOULD (soft), or MAY (optional) in the requirement"
                        .into(),
                ),
            });
        } else {
            has_rfc2119 = true;
        }
    }

    if !has_rfc2119 {
        warnings.push(CheckItem {
            severity: "warning".into(),
            check: "no_rfc2119_any".into(),
            element: "Plan".into(),
            location: "specs/".into(),
            detail: "No requirements use RFC 2119 keywords (SHALL/MUST/SHOULD/MAY)".into(),
            fix: Some(
                "Add SHALL/MUST/SHOULD/MAY constraints to make requirements verifiable".into(),
            ),
        });
    }

    (blockers, warnings, Vec::new())
}

fn check_task_references(plan: &PlanIR) -> (Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let warnings = Vec::new();

    let task_ids: Vec<String> = plan.tasks.iter().map(|t| t.id.clone()).collect();

    for req in &plan.requirements {
        let refs = translator::extract_task_refs_bare(&req.statement, &task_ids);
        for ref_id in refs {
            if !task_ids.contains(&ref_id) {
                blockers.push(CheckItem {
                    severity: "blocker".into(),
                    check: "bad_task_reference".into(),
                    element: format!("Requirement '{}'", req.id),
                    location: format!("{}:{}", req.source.file, req.source.start_line),
                    detail: format!("References task '{}' but no such task exists", ref_id),
                    fix: Some(format!(
                        "Change '{}' to a valid task ID: {:?}",
                        ref_id,
                        task_ids.iter().take(5).collect::<Vec<_>>()
                    )),
                });
            }
        }
    }

    (blockers, warnings)
}

fn check_classifiability(
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
        // MAY requirements are informational — emit as INFO, not blockers
        if req.strength == crate::ir::Rfc2119Strength::May {
            info.push(CheckItem {
                severity: "info".into(),
                check: "may_requirement".into(),
                element: format!("Requirement '{}'", req.id),
                location: format!("{}:{}", req.source.file, req.source.start_line),
                detail: format!(
                    "MAY '{}' is informational — not verified by model checking",
                    truncate(&req.statement, 80)
                ),
                fix: None,
            });
            continue;
        }
        let cat = translator::classify(&req.statement);

        // Detect PatternUngrounded: temporal pattern found but no task references
        let cat = if cat != ConstraintCategory::NonFormalizable
            && cat != ConstraintCategory::PatternUngrounded
        {
            let refs = translator::extract_task_refs_bare(&req.statement, &task_ids);
            if refs.is_empty() {
                // Has temporal pattern but no task references — pattern without grounding
                ConstraintCategory::PatternUngrounded
            } else {
                cat
            }
        } else {
            cat
        };

        if cat == ConstraintCategory::NonFormalizable {
            non_formalizable_count += 1;
            blockers.push(CheckItem {
                severity: "blocker".into(),
                check: "non_formalizable".into(),
                element: format!("Requirement '{}'", req.id),
                location: format!("{}:{}", req.source.file, req.source.start_line),
                detail: format!(
                    "SHALL '{}' does not match any temporal category",
                    truncate(&req.statement, 80)
                ),
                fix: Some(
                    "Rewrite as: sequential, exclusive, conditional, concurrent, or global constraint"
                        .into(),
                ),
            });
        } else if cat == ConstraintCategory::PatternUngrounded {
            // Temporal pattern detected but no task references to ground it
            // Severity depends on strictness profile (applied later by apply_strictness)
            // Always start as blocker, let apply_strictness adjust based on profile
            formalizable_count += 1;
            blockers.push(CheckItem {
                severity: "blocker".into(),
                check: "pattern_ungrounded".into(),
                element: format!("Requirement '{}'", req.id),
                location: format!("{}:{}", req.source.file, req.source.start_line),
                detail: format!(
                    "SHALL '{}' has a temporal pattern but no task references — add task IDs for model verification",
                    truncate(&req.statement, 80)
                ),
                fix: Some(
                    "Add task ID references (e.g., T1.2) to enable model verification".into(),
                ),
            });
        } else {
            formalizable_count += 1;
        }
    }

    if formalizable_count == 0 && non_formalizable_count > 0 {
        blockers.push(CheckItem {
            severity: "blocker".into(),
            check: "no_formalizable".into(),
            element: "Plan".into(),
            location: "specs/".into(),
            detail: "No requirements are classifiable into a temporal category".into(),
            fix: Some(
                "Rewrite all requirements using temporal constraint patterns: \
                 'X SHALL complete before Y' (sequential), \
                 'At most one X SHALL be active' (exclusive), \
                 'IF X fails THEN Y SHALL trigger' (conditional)"
                    .into(),
            ),
        });
    }

    info.push(CheckItem {
        severity: "info".into(),
        check: "classification_summary".into(),
        element: "Plan".into(),
        location: "specs/".into(),
        detail: format!(
            "{} formalizable, {} non-formalizable requirements",
            formalizable_count, non_formalizable_count
        ),
        fix: None,
    });

    (blockers, warnings, info)
}

fn check_scenarios(plan: &PlanIR) -> (Vec<CheckItem>, Vec<CheckItem>) {
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    for sc in &plan.scenarios {
        let has_when = sc.steps.iter().any(|s| s.kind == StepKind::When);
        let has_then = sc.steps.iter().any(|s| s.kind == StepKind::Then);

        if !has_when {
            warnings.push(CheckItem {
                severity: "warning".into(),
                check: "scenario_no_when".into(),
                element: format!("Scenario '{}'", sc.name),
                location: format!("{}:{}", sc.source.file, sc.source.start_line),
                detail: "Scenario missing WHEN step".into(),
                fix: Some("Add '- **WHEN** ...' to the scenario".into()),
            });
        }
        if !has_then {
            warnings.push(CheckItem {
                severity: "warning".into(),
                check: "scenario_no_then".into(),
                element: format!("Scenario '{}'", sc.name),
                location: format!("{}:{}", sc.source.file, sc.source.start_line),
                detail: "Scenario missing THEN step".into(),
                fix: Some("Add '- **THEN** ... SHALL ...' to the scenario".into()),
            });
        }

        // Check THEN contains SHALL
        if has_then {
            for step in &sc.steps {
                if step.kind == StepKind::Then || step.kind == StepKind::And {
                    let strength = crate::parser::detect_rfc2119(&step.text);
                    if strength == Rfc2119Strength::None {
                        warnings.push(CheckItem {
                            severity: "warning".into(),
                            check: "then_no_shall".into(),
                            element: format!("Scenario '{}'", sc.name),
                            location: format!("{}:{}", sc.source.file, step.source.start_line),
                            detail: format!("{:?} step has no RFC 2119 keyword", step.kind),
                            fix: Some("Add SHALL/MUST/SHOULD to the step".into()),
                        });
                    }
                }
            }
        }
    }

    info.push(CheckItem {
        severity: "info".into(),
        check: "scenario_count".into(),
        element: "Plan".into(),
        location: "specs/".into(),
        detail: format!(
            "Found {} scenarios across all spec files",
            plan.scenarios.len()
        ),
        fix: None,
    });

    (warnings, info)
}

fn check_diversity(plan: &PlanIR) -> Vec<CheckItem> {
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

    // Check for diversity issues
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
        info.push(CheckItem {
            severity: "info".into(),
            check: "low_diversity".into(),
            element: "Plan".into(),
            location: "specs/".into(),
            detail: format!(
                "Constraint distribution: {}. Consider adding other constraint types for stronger verification",
                summary.join(", ")
            ),
            fix: Some("Add exclusive (mutex), conditional (if-then), or concurrent constraints".into()),
        });
    }

    info.push(CheckItem {
        severity: "info".into(),
        check: "constraint_diversity".into(),
        element: "Plan".into(),
        location: "specs/".into(),
        detail: format!("Constraint distribution: {}", summary.join(", ")),
        fix: None,
    });

    info
}

/// Check that every task is referenced by at least one SHALL requirement.
/// In single-file/stdin mode (is_openspec=false), this check is downgraded to INFO
/// since it's expected that standalone files may not have full coverage.
fn check_task_coverage(plan: &PlanIR, is_openspec: bool) -> (Vec<CheckItem>, Vec<CheckItem>) {
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    // Collect all task IDs mentioned in requirement statements
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
            // In single-file/stdin mode, downgrade to INFO
            let severity = if is_openspec { "warning" } else { "info" };
            let item = CheckItem {
                severity: severity.into(),
                check: "task_not_covered".into(),
                element: format!("T{} ({})", task.id, task.description),
                location: format!("{}:{}", task.source.file, task.source.start_line),
                detail: format!(
                    "Task T{} is not referenced by any SHALL requirement — its behavior is unchecked.",
                    task.id
                ),
                fix: Some(format!(
                    "Add a SHALL in specs/ that references T{} with a temporal keyword (BEFORE, CONCURRENTLY, etc.).",
                    task.id
                )),
            };
            if severity == "warning" {
                warnings.push(item);
            } else {
                info.push(item);
            }
        }
    }

    info.push(CheckItem {
        severity: "info".into(),
        check: "task_coverage".into(),
        element: "Plan".into(),
        location: "tasks.md".into(),
        detail: format!(
            "{}/{} tasks are covered by SHALL requirements",
            plan.tasks.len() - uncovered,
            plan.tasks.len()
        ),
        fix: None,
    });

    (warnings, info)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

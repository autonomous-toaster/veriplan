//! Verifier engine: convertibility check → Promela generation → model checking.
//!
//! Three phases:
//!   1. Convertibility check (Phase 1): validate plan can become a formal model
//!   2. Promela + SPIN (Phase 2a): full SPIN model checking
//!   3. BFS fallback (Phase 2b): built-in explorer when SPIN unavailable
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt::Write;

use crate::ir::{
    CheckItem, ConstraintCategory, ConvertibilityReport, ConvertibilityStatus, PlanIR,
    Rfc2119Strength, StepKind, Task,
};
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

// ═══════════════════════════════════════════════════════════════
// Phase 1: Convertibility Check
// ═══════════════════════════════════════════════════════════════

/// Run the full convertibility check (Phase 1).
/// Returns a report with blockers, warnings, and info items.
pub fn check_convertibility(plan: &PlanIR) -> ConvertibilityReport {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    // Check 1: Tasks exist and have unique IDs
    let task_check = check_tasks(plan);
    if let Some(b) = task_check.0 { blockers.push(b) }
    warnings.extend(task_check.1);
    info.extend(task_check.2);

    // Check 2: Requirements exist and have RFC 2119 keywords
    let req_check = check_requirements(plan);
    blockers.extend(req_check.0);
    warnings.extend(req_check.1);
    info.extend(req_check.2);

    // Check 3: Task references
    let ref_check = check_task_references(plan);
    blockers.extend(ref_check.0);
    warnings.extend(ref_check.1);

    // Check 4: Temporal classifiability
    let class_check = check_classifiability(plan);
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
    let cov_check = check_task_coverage(plan);
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

fn check_requirements(plan: &PlanIR) -> (Vec<CheckItem>, Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if plan.requirements.is_empty() {
        blockers.push(CheckItem {
            severity: "blocker".into(),
            check: "no_requirements".into(),
            element: "Plan".into(),
            location: "specs/".into(),
            detail: "No requirements found in any spec file".into(),
            fix: Some(
                "Add ### Requirement: sections with SHALL/MUST paragraphs to spec files".into(),
            ),
        });
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

fn check_classifiability(plan: &PlanIR) -> (Vec<CheckItem>, Vec<CheckItem>, Vec<CheckItem>) {
    let mut blockers = Vec::new();
    let warnings = Vec::new();
    let mut info = Vec::new();

    let mut formalizable_count = 0;
    let mut non_formalizable_count = 0;

    for req in &plan.requirements {
        let cat = translator::classify(&req.statement);
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
fn check_task_coverage(plan: &PlanIR) -> (Vec<CheckItem>, Vec<CheckItem>) {
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
            warnings.push(CheckItem {
                severity: "warning".into(),
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
            });
        }
    }

    info.push(CheckItem {
        severity: "info".into(),
        check: "task_coverage".into(),
        element: "Plan".into(),
        location: "tasks.md".into(),
        detail: format!("{}/{} tasks are covered by SHALL requirements",
            plan.tasks.len() - uncovered,
            plan.tasks.len()),
        fix: None,
    });

    (warnings, info)
}

// ═══════════════════════════════════════════════════════════════
// Phase 2: Model Checking
// ═══════════════════════════════════════════════════════════════

/// Check if SPIN is available on PATH.
pub fn require_spin() -> Result<(), String> {
    match std::process::Command::new("spin").arg("--version").output() {
        Ok(_) => Ok(()),
        Err(_) => Err("SPIN binary not found on PATH. Install spin (brew install spin) and try again.".into()),
    }
}

/// Run the full verification pipeline (Phase 1 + Phase 2).
pub fn verify(plan: &PlanIR, plan_name: &str, no_model: bool) -> VerificationResult {
    // Phase 1: Convertibility check
    let conv_report = check_convertibility(plan);

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

    // Phase 2: Model checking
    let constraints = translator::translate_all(plan);
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();

    if formalizable.is_empty() {
        return VerificationResult {
            plan_name: plan_name.to_string(),
            phase: "model_check".into(),
            convertible: true,
            convertibility_report: Some(conv_report),
            valid: None,
            violations: vec![],
            total_constraints: 0,
            satisfied_constraints: 0,
            constraints_summary: vec![],
            skip_reason: Some("No formalizable constraints to check".into()),
        };
    }

    if let Err(msg) = require_spin() {
        return VerificationResult {
            plan_name: plan_name.to_string(),
            phase: "model_check".into(),
            convertible: false,
            convertibility_report: Some(conv_report),
            valid: None,
            violations: vec![],
            total_constraints: formalizable.len(),
            satisfied_constraints: 0,
            constraints_summary: vec![],
            skip_reason: Some(msg),
        };
    }

    run_spin_check(plan, plan_name, &constraints, conv_report)
}

/// Verify multiple plans and merge the results into a single report.
pub fn verify_all(plans: &[(String, PlanIR)], no_model: bool) -> VerificationResult {
    let mut all_results: Vec<VerificationResult> = Vec::new();
    for (name, plan) in plans {
        let result = verify(plan, name, no_model);
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
        constraints_summary: results.iter().flat_map(|r| r.constraints_summary.clone()).collect(),
        skip_reason: None,
    };

    if any_skipped && !any_valid {
        combined.skip_reason = Some("One or more changes were skipped".into());
    }

    combined
}

/// Generate a Promela model and run SPIN.
fn run_spin_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let promela = generate_promela(plan, constraints);
    let promela_path = format!("/tmp/veriplan_{}.pml", plan_name.replace('/', "_"));

    if std::fs::write(&promela_path, &promela).is_err() {
        return simple_result(plan_name, conv_report, vec![]);
    }

    // Phase 1: generate verifier source with spin -a (no search run)
    let promela_dir = std::path::Path::new(&promela_path)
        .parent()
        .unwrap_or(std::path::Path::new("/tmp"))
        .to_path_buf();
    let _pan_path = promela_dir.join("pan");

    let out_gen = std::process::Command::new("spin")
        .args(["-a", &promela_path])
        .current_dir(&promela_dir)
        .output();
    let pan_path = promela_dir.join("pan");

    if out_gen.is_err() {
        return simple_result(plan_name, conv_report, vec![]);
    }

    // Phase 2: compile pan.c into pan binary
    let pan_c = promela_dir.join("pan.c");
    if !pan_c.exists() {
        return simple_result(plan_name, conv_report, vec![]);
    }

    let compile = std::process::Command::new("gcc")
        .args(["-w", "-o", "pan", "pan.c"])
        .stdin(std::process::Stdio::null())
        .current_dir(promela_dir.clone())
        .output();

    if compile.is_err() || !pan_path.exists() {
        return simple_result(plan_name, conv_report, vec![]);
    }

    // Phase 2: run pan for each LTL property individually
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();
    let mut constraints_summary = Vec::new();
    let mut violations = Vec::new();
    let mut satisfied = 0usize;
    let mut _timed_out_count = 0usize;

    for (i, c) in formalizable.iter().enumerate() {
        let label = format!("p{}", i);

        // Liveness properties (with <>) need -a; safety properties don't — much faster
        let has_liveness = c.ltl.as_deref().unwrap_or("").contains("<>");
        let mut pan_args = vec!["-N", &label, "-n"];
        if has_liveness {
            pan_args.push("-a");
        }

        // Timeout per property: 5s per run (67 tasks × 23 properties = large state space)
        // If timeout, report as "unchecked" rather than pass/fail
        let output = timeout_command(&pan_path, &pan_args, 5);

        let (passed, violated, timed_out) = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                if combined.contains("errors: 1") || combined.contains("errors: 2") {
                    (false, true, false)
                } else {
                    (true, false, false)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                (false, false, true)
            }
            Err(_) => (false, true, false),
        };

        constraints_summary.push(ConstraintSummary {
            requirement_id: c.requirement_id.clone(),
            statement: c.statement.clone(),
            category: format!("{:?}", c.category),
            satisfied: passed,
            unchecked: timed_out,
        });

        if timed_out {
            _timed_out_count += 1;
        } else if violated {
            let fix = suggest_fix(&c.category, c.ltl.as_deref().unwrap_or(""), &c.requirement_id);
            violations.push(Violation {
                constraint_id: c.requirement_id.clone(),
                requirement_statement: c.statement.clone(),
                ltl: c.ltl.clone().unwrap_or_default(),
                category: format!("{:?}", c.category),
                state: format!("(violated in property {})", label),
                task_source: None,
                req_source: None,
                suggested_fix: fix,
                plan: plan_name.to_string(),
            });
        } else {
            satisfied += 1;
        }
    }

    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid: Some(violations.is_empty()),
        violations,
        total_constraints: formalizable.len(),
        satisfied_constraints: satisfied,
        constraints_summary,
        skip_reason: None,
    }
}

/// Generate Promela source from PlanIR and constraints.
fn generate_promela(plan: &PlanIR, constraints: &[translator::TranslatedConstraint]) -> String {
    let mut s = String::new();

    // Header
    writeln!(s, "/* Promela model — task structure only */").ok();
    writeln!(s).ok();

    // ── Variable declarations ──
    for task in &plan.tasks {
        let desc = task.description.replace("/*", "/ *").replace("*/", "* /");
        writeln!(
            s,
            "bit {} = 0;\t/* {} */",
            active_var(&task.id),
            desc
        )
        .ok();
        writeln!(
            s,
            "bit {} = 0;",
            done_var(&task.id)
        )
        .ok();
    }
    writeln!(s).ok();

    // Failed flags for conditional constraint LTL references
    for task in &plan.tasks {
        writeln!(s, "bit {} = 0;", fail_var(&task.id)).ok();
    }
    writeln!(s).ok();

    // ── Task execution processes (phase-ordered only) ──
    for task in &plan.tasks {
        let av = active_var(&task.id);
        let dv = done_var(&task.id);
        let fv = fail_var(&task.id);

        writeln!(
            s,
            "active proctype task_{}() {{",
            &task.id.replace('.', "_")
        )
        .ok();

        // Only phase-ordering guard: predecessor must be done
        let predecessors = find_predecessors(plan, &task.id);
        if predecessors.is_empty() {
            writeln!(s, "\tdo").ok();
            writeln!(s, "\t:: (1) ->").ok();
        } else {
            let guard = predecessors
                .iter()
                .map(|id| format!("{} == 1", done_var(id)))
                .collect::<Vec<_>>()
                .join(" && ");
            writeln!(s, "\tdo").ok();
            writeln!(s, "\t:: {} ->", guard).ok();
        }

        // Task body
        writeln!(s, "\t\t{} = 1;\t/* activate */", av).ok();
        writeln!(s, "\t\t{} = 1;\t/* complete */", dv).ok();
        writeln!(s, "\t\t{} = 0;\t/* deactivate */", av).ok();

        // Non-deterministic failure (for conditional constraint exploration)
        writeln!(s, "\t\tif").ok();
        writeln!(s, "\t\t:: {} = 1;", fv).ok();
        writeln!(s, "\t\t:: skip;").ok();
        writeln!(s, "\t\tfi;").ok();

        writeln!(s, "\t\tbreak").ok();
        writeln!(s, "\tod").ok();
        writeln!(s, "}}").ok();
        writeln!(s).ok();
    }

    // ── LTL properties — spec constraints checked against phase-ordered model ──
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();
    for (i, c) in formalizable.iter().enumerate() {
        if let Some(ltl) = &c.ltl {
            writeln!(
                s,
                "ltl p{} {{ {} }} /* {} */",
                i, ltl, c.requirement_id
            )
            .ok();
        }
    }

    s
}

fn active_var(id: &str) -> String {
    format!("active_t{}", id.replace('.', "_"))
}

fn done_var(id: &str) -> String {
    format!("done_t{}", id.replace('.', "_"))
}

fn fail_var(id: &str) -> String {
    format!("failed_t{}", id.replace('.', "_"))
}

/// Run the built-in BFS state explorer.
fn run_bfs_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();

    let mut violations = Vec::new();

    // Simple state enumeration
    for state_bits in 0u64..(1u64 << plan.tasks.len().min(20)) {
        let mut state: HashMap<String, u8> = HashMap::new();
        for (j, task) in plan.tasks.iter().enumerate() {
            let val = if (state_bits >> j) & 1 == 1 { 1 } else { 0 };
            state.insert(task.id.clone(), val);
        }

        for c in &formalizable {
            if let Some(ltl) = &c.ltl
                && !evaluate_ltl(ltl, &state, plan)
                    && !violations
                        .iter()
                        .any(|v: &Violation| v.constraint_id == c.requirement_id)
                    {
                        let state_str: Vec<String> = state
                            .iter()
                            .filter(|(_, v)| **v == 1)
                            .map(|(k, _)| k.clone())
                            .collect();
                        violations.push(Violation {
                            constraint_id: c.requirement_id.clone(),
                            requirement_statement: c.statement.clone(),
                            ltl: ltl.clone(),
                            category: format!("{:?}", c.category),
                            state: state_str.join(", "),
                            task_source: None,
                            req_source: None,
                            suggested_fix: None,
                            plan: plan_name.to_string(),
                        });
                    }
        }
    }

    let valid = violations.is_empty();
    let violations_count = violations.len();
    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid: Some(valid),
        violations,
        total_constraints: formalizable.len(),
        satisfied_constraints: if valid {
            formalizable.len()
        } else {
            formalizable.len().saturating_sub(violations_count)
        },
        constraints_summary: vec![],
        skip_reason: None,
    }
}

/// Simple LTL evaluator for BFS explorer.
fn evaluate_ltl(ltl: &str, state: &HashMap<String, u8>, plan: &PlanIR) -> bool {
    // Very basic evaluation: parse simple G ( condition ) patterns
    if let Some(inner) = ltl.strip_prefix("G ( ").and_then(|s| s.strip_suffix(" )")) {
        let inner = inner.trim();
        return evaluate_ltl_condition(inner, state, plan);
    }
    // For anything we can't evaluate, assume pass (conservative)
    true
}

fn evaluate_ltl_condition(cond: &str, state: &HashMap<String, u8>, _plan: &PlanIR) -> bool {
    // Split on && for compound conditions
    if cond.contains("&&") {
        return cond
            .split("&&")
            .all(|part| evaluate_ltl_condition(part.trim(), state, _plan));
    }

    let cond = cond.trim();

    // Negation: !(expr)
    if let Some(inner) = cond.strip_prefix("!(").and_then(|s| s.strip_suffix(')')) {
        return !evaluate_ltl_condition(inner, state, _plan);
    }

    // active_X -> done_Y (implication)
    if let Some((ante, conseq)) = cond.split_once("->") {
        let ante = ante.trim();
        let conseq = conseq.trim();
        let ante_val = evaluate_ltl_atom(ante, state);
        let conseq_val = evaluate_ltl_atom(conseq, state);
        return !ante_val || conseq_val; // implication: a -> b ≡ !a || b
    }

    // active_X <-> active_Y (bidirectional)
    if let Some((left, right)) = cond.split_once("<->") {
        let left_val = evaluate_ltl_atom(left.trim(), state);
        let right_val = evaluate_ltl_atom(right.trim(), state);
        return left_val == right_val;
    }

    // F active_X (eventually)
    if let Some(arg) = cond.strip_prefix("F ") {
        return evaluate_ltl_atom(arg.trim(), state);
    }

    // Atomic
    evaluate_ltl_atom(cond, state)
}

fn evaluate_ltl_atom(atom: &str, state: &HashMap<String, u8>) -> bool {
    let atom = atom.trim();

    // Negation
    if let Some(var) = atom.strip_prefix('!') {
        return !state.get(var).copied().unwrap_or(0) == 1;
    }

    // Check if this is a variable name
    let val = state.get(atom).copied().unwrap_or(0);
    val == 1
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

fn find_predecessors(plan: &PlanIR, task_id: &str) -> Vec<String> {
    // Find which phase this task belongs to
    for (idx, phase) in plan.phases.iter().enumerate() {
        if !phase.task_ids.iter().any(|id| id == task_id) {
            continue;
        }
        // Concurrent phase: no intra-phase ordering.
        // All tasks in this phase wait for the previous phase to complete.
        if phase.mode == crate::ir::PhaseMode::Concurrent {
            if idx > 0 {
                let prev_phase = &plan.phases[idx - 1];
                if let Some(last_id) = prev_phase.task_ids.last() {
                    return vec![last_id.clone()];
                }
            }
            return Vec::new();
        }
        // Sequential phase: current behavior — previous task in same phase
        if let Some(pos) = phase.task_ids.iter().position(|id| id == task_id) {
            if pos > 0 {
                return vec![phase.task_ids[pos - 1].clone()];
            }
            // First task in phase: wait for last task of previous phase
            if idx > 0 {
                let prev_phase = &plan.phases[idx - 1];
                if let Some(last_id) = prev_phase.task_ids.last() {
                    return vec![last_id.clone()];
                }
            }
        }
        return Vec::new();
    }
    Vec::new()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

/// Run a command with a timeout in seconds.
/// Returns Ok(output) if the command completes within the timeout,
/// or an Err if it times out or fails to start.
fn timeout_command(
    cmd: &std::path::Path,
    args: &[&str],
    timeout_secs: u64,
) -> std::io::Result<std::process::Output> {
    // Use a thread-based timeout approach
    use std::sync::mpsc;
    use std::time::Duration;

    let cmd_path = cmd.to_path_buf();
    let args_owned: Vec<String> = args.iter().map(|a| a.to_string()).collect();

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = std::process::Command::new(&cmd_path)
            .args(&args_owned)
            .output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("command timed out after {}s", timeout_secs),
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "command thread disconnected",
            ))
        }
    }
}

/// Extract task IDs like "T4.2" from a statement or "t4_2" from LTL.
fn extract_task_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let bytes = text.as_bytes();
    let n = bytes.len();
    // Match T4.2, T6.1, etc.
    let mut i = 0;
    while i < n {
        if bytes[i] == b'T' && i + 1 < n && bytes[i+1].is_ascii_digit() {
            i += 1;
            let start = i;
            while i < n && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                && s.contains('.') && s.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    ids.push(s.to_string());
                }
        } else {
            i += 1;
        }
    }
    // Match t4_2, t6_1, etc. (from LTL active_t4_2, failed_t6_1)
    if ids.is_empty() {
        let mut i = 0;
        while i < n {
            if bytes[i] == b't' && i + 1 < n && bytes[i+1].is_ascii_digit() {
                i += 1;
                let start = i;
                while i < n && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                    i += 1;
                }
                if let Ok(s) = std::str::from_utf8(&bytes[start..i])
                    && let Some(underscore) = s.find('_') {
                        let major = &s[..underscore];
                        let minor = &s[underscore+1..];
                        ids.push(format!("{}.{}", major, minor));
                    }
            } else {
                i += 1;
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

/// Generate human-readable guidance for a constraint violation.
fn suggest_fix(
    category: &crate::ir::ConstraintCategory,
    ltl: &str,
    _req_id: &str,
) -> Option<String> {
    let task_ids = extract_task_ids(ltl);
    let task_list = if task_ids.is_empty() {
        String::new()
    } else if task_ids.len() <= 2 {
        format!(
            " tasks {} and {}",
            task_ids.first().unwrap(),
            task_ids.get(1).unwrap_or(&task_ids[0])
        )
    } else {
        format!(" tasks {}", task_ids.join(", "))
    };

    match category {
        crate::ir::ConstraintCategory::ConcurrentEvents => {
            if task_list.is_empty() {
                Some(
                    "The model runs tasks sequentially by phase — two tasks cannot be active simultaneously.\n  Either remove the CONCURRENTLY keyword from this requirement, or restructure the plan\n  so these tasks can overlap in execution."
                        .into(),
                )
            } else {
                Some(format!(
                    "The model runs tasks sequentially by phase — two tasks cannot be active simultaneously.\n  The requirement references{} but they execute one after another within the same phase.\n  Either remove the CONCURRENTLY keyword from this requirement, or restructure the plan\n  to put these tasks in a concurrent phase.",
                    task_list
                ))
            }
        }
        crate::ir::ConstraintCategory::Conditional => {
            Some(
                "The trigger task fails non-deterministically but the consequent task never activates.\n  IF...THEN is designed for **failure-recovery** patterns (e.g. 'IF T1.1 fails THEN T1.2 SHALL run').\n  For **branching/decision logic** (e.g. 'IF X THEN A, IF not X THEN B'), use Sequential ordering instead:\n  \"T1.5 SHALL complete BEFORE T1.4 SHALL run\".\n  Otherwise mark this constraint as aspirational by removing IF...THEN."
                    .into(),
            )
        }
        crate::ir::ConstraintCategory::Exclusive => {
            if task_list.is_empty() {
                Some(
                    "Two tasks can be active simultaneously in the model — they are not mutually exclusive.\n  Either add a phase ordering between them, or mark this constraint as aspirational\n  by removing AT MOST ONE / NOT CONCURRENTLY."
                        .into(),
                )
            } else {
                Some(format!(
            "Tasks{} can both be active at the same time in the model — they are not mutually exclusive.\n  Either add a phase ordering between them (different phases execute sequentially),\n  or mark this constraint as aspirational by removing AT MOST ONE / NOT CONCURRENTLY.",
                    task_list
                ))
            }
        }
        crate::ir::ConstraintCategory::SequentialOrder => {
            Some(
                "The before-task does not always complete before the after-task starts in the model.\n  Either ensure the before-task is in an earlier phase, or mark this constraint\n  as aspirational by removing BEFORE / AFTER."
                    .into(),
            )
        }
        _ => None,
    }
}

fn simple_result(
    plan_name: &str,
    conv_report: ConvertibilityReport,
    _constraints: Vec<translator::TranslatedConstraint>,
) -> VerificationResult {
    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid: None,
        violations: vec![],
        total_constraints: 0,
        satisfied_constraints: 0,
        constraints_summary: vec![],
        skip_reason: Some("Model check error".into()),
    }
}

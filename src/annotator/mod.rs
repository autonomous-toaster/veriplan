//! Annotator: maps model checker results to human-readable + JSON output.

use crate::checker::{VerificationResult, Violation};
use crate::ir::PlanIR;

/// Helper: extract task IDs like "4.2" from LTL formula (active_t4_2 → "4.2").
fn task_ids_from_ltl(ltl: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let bytes = ltl.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == b't' && i + 2 < n && bytes[i+1].is_ascii_digit() && bytes[i+2] == b'_' {
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
    ids.sort();
    ids.dedup();
    ids
}

/// Parse trigger and consequent from a Conditional LTL formula.
/// Conditional LTL: `[] ( failed_t<X>_<Y> -> <> active_t<A>_<B> )`
/// Returns (trigger_id, consequent_id) like ("1.4", "1.5").
fn parse_conditional_ltl(ltl: &str) -> Option<(String, String)> {
    let ltl_bytes = ltl.as_bytes();
    let n = ltl_bytes.len();

    // Find "failed_t" and extract trigger
    let failed_idx = ltl.find("failed_t")?;
    let mut i = failed_idx + 8; // skip "failed_t"
    // read digits until '_'
    let mut major = String::new();
    let mut minor = String::new();
    while i < n && ltl_bytes[i].is_ascii_digit() {
        major.push(ltl_bytes[i] as char);
        i += 1;
    }
    // skip '_'
    if i < n && ltl_bytes[i] == b'_' { i += 1; }
    while i < n && ltl_bytes[i].is_ascii_digit() {
        minor.push(ltl_bytes[i] as char);
        i += 1;
    }
    if major.is_empty() || minor.is_empty() { return None; }
    let trigger = format!("{}.{}", major, minor);

    // Find "active_t" and extract consequent
    let active_idx = ltl.find("active_t")?;
    let mut i = active_idx + 8; // skip "active_t"
    let mut major = String::new();
    let mut minor = String::new();
    while i < n && ltl_bytes[i].is_ascii_digit() {
        major.push(ltl_bytes[i] as char);
        i += 1;
    }
    // skip '_'
    if i < n && ltl_bytes[i] == b'_' { i += 1; }
    while i < n && ltl_bytes[i].is_ascii_digit() {
        minor.push(ltl_bytes[i] as char);
        i += 1;
    }
    if major.is_empty() || minor.is_empty() { return None; }
    let consequent = format!("{}.{}", major, minor);

    Some((trigger, consequent))
}

/// Build phase context string like "T4.2 (Phase 4), T4.4 (Phase 4)".
fn build_phase_context(ltl: &str, plan: &PlanIR) -> Option<String> {
    let task_ids = task_ids_from_ltl(ltl);
    if task_ids.is_empty() {
        return None;
    }
    // Build a map: task_id → phase
    let mut phases: std::collections::BTreeMap<String, &str> = std::collections::BTreeMap::new();
    for t in &plan.tasks {
        if task_ids.contains(&t.id) {
            phases.insert(t.id.clone(), t.phase.as_str());
        }
    }
    if phases.is_empty() {
        return None;
    }
    let parts: Vec<String> = phases
        .into_iter()
        .map(|(id, phase)| format!("T{} ({})", id, phase))
        .collect();
    Some(parts.join(", "))
}

/// Build category breakdown string from violations.
fn category_breakdown(violations: &[AnnotatedViolation]) -> String {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for v in violations {
        *counts.entry(v.category.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(cat, n)| format!("{}: {}", cat, n))
        .collect::<Vec<_>>()
        .join(", ")
}

/// An annotated violation with source locations resolved.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotatedViolation {
    pub constraint_id: String,
    pub requirement_statement: String,
    pub ltl: String,
    pub category: String,
    pub state: String,
    pub source_file: Option<String>,
    pub source_line: Option<usize>,
    pub suggested_fix: Option<String>,
    /// Phase context extracted from plan (e.g., "T4.2 (Phase 4)")
    pub phase_context: Option<String>,
}

/// Annotate a verification result with source locations from PlanIR.
pub fn annotate(result: &VerificationResult, plan: &PlanIR) -> Vec<AnnotatedViolation> {
    result
        .violations
        .iter()
        .map(|v| {
            let (src_file, src_line) = resolve_source(v, plan);
            let phase_ctx = build_phase_context(&v.ltl, plan);
            AnnotatedViolation {
                constraint_id: v.constraint_id.clone(),
                requirement_statement: v.requirement_statement.clone(),
                ltl: v.ltl.clone(),
                category: v.category.clone(),
                state: v.state.clone(),
                source_file: src_file.or_else(|| v.task_source.clone()),
                source_line: src_line,
                suggested_fix: v.suggested_fix.clone(),
                phase_context: phase_ctx,
            }
        })
        .collect()
}

fn resolve_source(v: &Violation, plan: &PlanIR) -> (Option<String>, Option<usize>) {
    // Try to find the requirement's source
    for req in &plan.requirements {
        if req.id == v.constraint_id {
            return (Some(req.source.file.clone()), Some(req.source.start_line));
        }
    }
    (None, None)
}

/// Format a human-readable report string.
pub fn format_human(
    result: &VerificationResult,
    annotated: &[AnnotatedViolation],
    plan: &PlanIR,
    verbose: bool,
) -> String {
    let mut output = String::new();
    let plan_name = &result.plan_name;

    match result.phase.as_str() {
        "convertibility" => {
            if let Some(ref report) = result.convertibility_report {
                output.push_str(&format!(
                    "📋 Plan: {} — Convertibility Check\n\n",
                    plan_name
                ));

                if !report.blockers.is_empty() {
                    output.push_str(&format!("✗ {} blocker(s):\n", report.blockers.len()));
                    for b in &report.blockers {
                        output.push_str(&format!("  [BLOCKER] {} at {}\n", b.element, b.location));
                        output.push_str(&format!("           {}\n", b.detail));
                        if let Some(ref fix) = b.fix {
                            output.push_str(&format!("           Fix: {}\n", fix));
                        }
                    }
                    output.push_str(
                        "\n  Rephrase the spec following the fix suggestions and re-run.\n",
                    );
                }

                if !report.warnings.is_empty() {
                    output.push_str(&format!("\n⚠ {} warning(s):\n", report.warnings.len()));
                    for w in &report.warnings {
                        output.push_str(&format!("  [WARNING] {} at {}\n", w.element, w.location));
                        output.push_str(&format!("            {}\n", w.detail));
                    }
                }

                if !report.info.is_empty() {
                    output.push_str(&format!("\nℹ {} info(s):\n", report.info.len()));
                    for i in &report.info {
                        output.push_str(&format!("  [INFO] {} {}\n", i.element, i.detail));
                    }
                }
            }
        }
        "model_check" | "full" => {
            let status = if result.valid.unwrap_or(false) {
                "✓ VALID"
            } else if result.skip_reason.is_some() {
                "⚠ SKIPPED"
            } else {
                "✗ INVALID"
            };

            output.push_str(&format!("Plan: {} — {}\n\n", plan_name, status));

            // Model explanation header: only when there are violations to interpret
            if !annotated.is_empty() {
                output.push_str(
                    "  Verification model: the plan's task-phase structure is modeled as a\n  state machine. Each spec constraint is checked as an LTL property\n  against this model. A violation means the spec demands behavior\n  that the plan structure cannot guarantee.\n\n",
                );
            }

            if let Some(ref reason) = result.skip_reason {
                output.push_str(&format!("  Model check skipped: {}\n", reason));
            } else if annotated.is_empty() {
                output.push_str("  All constraints satisfied.\n");
            } else {
                output.push_str(&format!(
                    "  {} violation(s) out of {} constraints:\n\n",
                    annotated.len(),
                    result.total_constraints
                ));
                for v in annotated {
                    output.push_str(&format!(
                        "  ⚠ {} (category: {})\n",
                        v.constraint_id, v.category
                    ));
                    output.push_str(&format!("     Statement: {}\n", v.requirement_statement));
                    output.push_str(&format!("     LTL: {}
", v.ltl));
                    // For Conditional violations, show trigger/consequent breakdown
                    if v.category == "conditional"
                        && let Some((trigger, consequent)) = parse_conditional_ltl(&v.ltl) {
                            output.push_str(&format!(
                                "     Trigger: T{} (when this task fails)\n     Consequent: T{} (this task should activate)\n",
                                trigger, consequent
                            ));
                        }
                    if let Some(ref ctx) = v.phase_context {
                        output.push_str(&format!("     Tasks: {}\n", ctx));
                    }
                    if let Some(ref file) = v.source_file {
                        let line = v.source_line.unwrap_or(0);
                        output.push_str(&format!("     At: {}:{}\n", file, line));
                    }
                    output.push_str(&format!("     State: {}\n", v.state));
                    if let Some(ref fix) = v.suggested_fix {
                        output.push_str(&format!("     Fix: {}\n", fix));
                    }
                    output.push('\n');
                }
            }

            if result.skip_reason.is_none() {
                let unchecked_count = result
                    .constraints_summary
                    .iter()
                    .filter(|cs| cs.unchecked)
                    .count();
                let violated_count =
                    result.total_constraints - result.satisfied_constraints - unchecked_count;
                output.push_str(&format!(
                    "  Satisfied: {} | Violated: {} | Unchecked: {} | Total: {}\n",
                    result.satisfied_constraints,
                    violated_count,
                    unchecked_count,
                    result.total_constraints
                ));

                // Per-category breakdown — only when there are violations
                let break_down = category_breakdown(annotated);
                if !break_down.is_empty() {
                    output.push_str(&format!("  Violations by category: {}\n", break_down));
                }

                // Per-constraint list: always show when invalid, only in verbose when valid
                if !result.constraints_summary.is_empty() {
                    let show_constraints = !annotated.is_empty() || verbose;
                    if show_constraints {
                        output.push_str("\n  Constraints:\n");
                        for cs in &result.constraints_summary {
                            let mark = if cs.unchecked {
                                "~"
                            } else if cs.satisfied {
                                "✓"
                            } else {
                                "✗"
                            };
                            output.push_str(&format!(
                                "    {}  {}  {}\n",
                                mark, cs.category, cs.requirement_id
                            ));
                        }
                    }
                }

                if verbose {
                    verbose_section(&mut output, plan, result);
                }
            }
        }
        _ => {
            output.push_str(&format!("Plan: {} — status unknown\n", plan_name));
        }
    }

    output
}

/// Append verbose debug information (tasks, requirements, constraints) to output.
fn verbose_section(output: &mut String, plan: &PlanIR, _result: &VerificationResult) {
    output.push('\n');
    // Tasks grouped by phase
    output.push_str("  ── Tasks ──\n");
    if plan.phases.is_empty() {
        for task in &plan.tasks {
            let loc = &task.source;
            output.push_str(&format!(
                "    {}  {}  ({}:{})\n",
                task.id, task.description, loc.file, loc.start_line
            ));
        }
    } else {
        for phase in &plan.phases {
            let phase_tasks: Vec<_> = plan
                .tasks
                .iter()
                .filter(|t| t.phase == phase.name)
                .collect();
            if !phase_tasks.is_empty() {
                output.push_str(&format!("    {}:\n", phase.name));
                for task in &phase_tasks {
                    output.push_str(&format!(
                        "      {}  {}\n",
                        task.id, task.description
                    ));
                }
            }
        }
    }

    // Requirements with classification
    output.push_str("\n  ── Requirements ──\n");
    for req in &plan.requirements {
        let strength_str = format!("{:?}", req.strength);
        let cat_str = format!("{:?}", req.category);
        let _loc = &req.source;
        output.push_str(&format!(
            "    {}  strength={}  category={}\n",
            req.id, strength_str, cat_str
        ));
    }
    output.push('\n');
}

/// Format a JSON report string.
pub fn format_json(
    result: &VerificationResult,
    annotated: &[AnnotatedViolation],
    _plan: &PlanIR,
    _verbose: bool,
) -> String {
    let json_output = serde_json::json!({
        "plan": result.plan_name,
        "phase": result.phase,
        "convertible": result.convertible,
        "valid": result.valid,
        "total_constraints": result.total_constraints,
        "satisfied_constraints": result.satisfied_constraints,
        "skip_reason": result.skip_reason,
        "constraints_summary": result.constraints_summary,
        "violations": annotated.iter().map(|v| {
            serde_json::json!({
                "constraint_id": v.constraint_id,
                "statement": v.requirement_statement,
                "ltl": v.ltl,
                "category": v.category,
                "state": v.state,
                "source_file": v.source_file,
                "source_line": v.source_line,
                "suggested_fix": v.suggested_fix,
            })
        }).collect::<Vec<_>>(),
        "convertibility_report": result.convertibility_report,
    });
    serde_json::to_string_pretty(&json_output).unwrap_or_default()
}

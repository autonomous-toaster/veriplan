//! Annotator — enrich violations with source locations, task context, and suggested fixes.

mod helpers;

use crate::checker::Violation;
use crate::ir::{CheckItem, Finding, Fixability, PlanIR};

pub use helpers::{
    build_phase_context, category_breakdown, parse_conditional_ltl, task_ids_from_ltl,
};
/// Flatten convertibility blockers/warnings/info and model-check violations
/// into one canonical `Vec<Finding>` (design D1/D5).
///
/// This is the unified output contract: every finding is always present in
/// both default JSON and human output, regardless of `--verbose`.
pub fn findings(
    result: &crate::checker::VerificationResult,
    annotated: &[AnnotatedViolation],
) -> Vec<Finding> {
    let mut out = Vec::new();

    if let Some(report) = &result.convertibility_report {
        for item in report
            .blockers
            .iter()
            .chain(report.warnings.iter())
            .chain(report.info.iter())
        {
            out.push(check_item_to_finding(item));
        }
    }

    for v in annotated {
        out.push(violation_to_finding(v));
    }

    out
}
/// Convert a `CheckItem` (convertibility blocker/warning/info) to a `Finding`.
fn check_item_to_finding(item: &CheckItem) -> Finding {
    let (file, line) = parse_location(&item.location);
    Finding {
        kind: item.kind.as_str().to_string(),
        severity: item.severity.clone(),
        file,
        line,
        column: 0,
        start: item.start,
        end: item.end,
        message: item.detail.clone(),
        suggestion: item.fix.clone(),
        replacement: item.replacement.clone(),
        fixability: item.fixability,
        op: item.op,
        requirement_id: extract_requirement_id(&item.element),
        advisory: item.severity == "info" || item.kind == crate::ir::Kind::ProseOther,
    }
}
/// Convert an annotated model-check violation to a `Finding`.
fn violation_to_finding(v: &AnnotatedViolation) -> Finding {
    let (file, line) = v
        .req_source
        .as_deref()
        .or(v.task_source.as_deref())
        .map(parse_location)
        .unwrap_or_else(|| (String::new(), 0));
    Finding {
        kind: v.violation.kind.as_str().to_string(),
        severity: "blocker".into(),
        file,
        line,
        column: 0,
        start: 0,
        end: 0,
        message: format!(
            "{}: {}",
            v.violation.constraint_id, v.violation.requirement_statement
        ),
        suggestion: v.violation.suggested_fix.clone(),
        replacement: None,
        fixability: Fixability::Structural,
        op: v.violation.op,
        requirement_id: Some(v.violation.constraint_id.clone()),
        advisory: false,
    }
}
/// Parse a "file:line" location string into (file, line).
fn parse_location(loc: &str) -> (String, usize) {
    if let Some((file, line)) = loc.rsplit_once(':')
        && let Ok(line) = line.parse::<usize>()
    {
        return (file.to_string(), line);
    }
    (loc.to_string(), 0)
}
/// Extract a requirement ID from an element string like "Requirement 'R1'".
fn extract_requirement_id(element: &str) -> Option<String> {
    element
        .strip_prefix("Requirement '")
        .and_then(|s| s.strip_suffix('\''))
        .map(|s| s.to_string())
}
/// Group identical findings by `kind` for compact human output (design D6).
/// Returns a list of (count, representative finding).
pub fn group_by_kind(findings: &[Finding]) -> Vec<(usize, &Finding)> {
    let mut groups: Vec<(usize, &Finding)> = Vec::new();
    for f in findings {
        if let Some((count, _)) = groups.iter_mut().find(|(_, g)| g.kind == f.kind) {
            *count += 1;
        } else {
            groups.push((1, f));
        }
    }
    groups
}
/// Annotated violation with additional context.
#[derive(Debug, Clone)]
pub struct AnnotatedViolation {
    pub violation: Violation,
    pub task_source: Option<String>,
    pub req_source: Option<String>,
    pub phase_context: Option<String>,
    pub trigger_task: Option<String>,
    pub consequent_task: Option<String>,
    pub category: String,
}
/// Annotate violations with source locations and context.
pub fn annotate(
    result: &crate::checker::VerificationResult,
    plans: &[(String, PlanIR)],
) -> Vec<AnnotatedViolation> {
    let mut annotated = Vec::new();

    for violation in &result.violations {
        let plan = plans
            .iter()
            .find(|(name, _)| name == &violation.plan)
            .map(|(_, p)| p)
            .or_else(|| plans.first().map(|(_, p)| p));

        let Some(plan) = plan else {
            continue;
        };

        let (task_source, req_source) = resolve_source(violation, plan);

        let phase_context = helpers::build_phase_context(&violation.ltl, plan);

        let (trigger_task, consequent_task) = if violation.category.contains("Conditional") {
            helpers::parse_conditional_ltl(&violation.ltl)
                .map(|(t, c)| (Some(t), Some(c)))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        annotated.push(AnnotatedViolation {
            violation: violation.clone(),
            task_source,
            req_source,
            phase_context,
            trigger_task,
            consequent_task,
            category: violation.category.clone(),
        });
    }

    annotated
}
/// Format verification result as human-readable text.
///
/// Always shows findings at default verbosity (design D5). Identical findings
/// are grouped by `kind` ("N× <kind>: <rephrase>") with one representative
/// location; `--verbose` expands grouped findings (design D6).
pub fn format_human(
    result: &crate::checker::VerificationResult,
    annotated: &[AnnotatedViolation],
    plans: &[(String, PlanIR)],
    verbose: bool,
) -> String {
    let mut output = String::new();

    let all_findings = findings(result, annotated);
    let status = status_label(result, &all_findings);
    // The backend is named in the verdict: two runs differing only in backend used to be
    // indistinguishable, and the engine decides what the number means.
    let engine = if result.backend.is_empty() {
        String::new()
    } else {
        format!(" [{}]", result.backend)
    };
    output.push_str(&format!(
        "Plan: {} — {}{}\n",
        result.plan_name, status, engine
    ));

    if let Some(reason) = &result.skip_reason {
        output.push_str(&format!("\n  Model check skipped: {}\n", reason));
    }

    if !all_findings.is_empty() {
        format_findings(&mut output, &all_findings, verbose);
    } else if result.convertible && result.valid == Some(true) {
        output.push_str("  All constraints satisfied.\n");
        output.push_str(&format!(
            "  Satisfied: {} | Violated: 0 | Unmodelled: {} | Unchecked: {} | Historical: {} | Total: {}\n",
            result.satisfied_constraints,
            result.unmodelled_constraints,
            result.unchecked_constraints,
            result.historical_constraints,
            result.total_constraints
        ));
    }

    if verbose {
        verbose_section(&mut output, plans, result);
    }

    output
}
/// Render findings, grouping identical ones by `kind` at default verbosity and
/// expanding them under `--verbose` (design D6).
fn format_findings(output: &mut String, all_findings: &[Finding], verbose: bool) {
    if verbose {
        // Expand every finding individually.
        for f in all_findings {
            output.push_str(&format!(
                "  [{}] {} at {}:{} — {}\n",
                f.severity.to_uppercase(),
                f.kind,
                f.file,
                f.line,
                f.message
            ));
            if let Some(s) = &f.suggestion {
                output.push_str(&format!("        Fix: {}\n", s));
            }
        }
        return;
    }
    // Default: group identical findings by `kind` ("N× <kind>: <rephrase>").
    let groups = group_by_kind(all_findings);
    for (count, f) in groups {
        let rephrase = f.suggestion.as_deref().unwrap_or(&f.message);
        output.push_str(&format!(
            "  {}× {}: {} (e.g. {}:{})\n",
            count, f.kind, rephrase, f.file, f.line
        ));
    }
}
fn status_label(
    result: &crate::checker::VerificationResult,
    all_findings: &[Finding],
) -> &'static str {
    if !result.convertible {
        "⚠ SKIPPED"
    } else if result.valid == Some(false) {
        "✗ INVALID"
    } else if result.valid == Some(true) {
        // The verdict is derived from the same flattened finding set that is
        // printed, so the label and the findings list can never contradict a
        // `blocker` (including prose blockers in Strict). This is the fix for
        // the original "✓ VALID" + "[BLOCKER] prose_other" contradiction.
        if all_findings.iter().any(|f| f.severity == "blocker") {
            "⚠ VALID (with blockers)"
        } else {
            "✓ VALID"
        }
    } else {
        "⚠ UNKNOWN"
    }
}
/// Format verification result as JSON.
///
/// Always emits a top-level `findings[]` array (regardless of `--verbose`),
/// per design D5. `--verbose` adds only supplementary lists (rephrase
/// directives, constraint summaries) and never changes which core findings
/// are present. **BREAKING**: the old top-level `convertibility_report` and
/// `violations` keys are dropped in favor of `findings[]`.
pub fn format_json(
    result: &crate::checker::VerificationResult,
    annotated: &[AnnotatedViolation],
    _plans: &[(String, PlanIR)],
    verbose: bool,
) -> String {
    let all_findings = findings(result, annotated);

    let mut output = serde_json::json!({
        "plan_name": result.plan_name,
        "phase": result.phase,
        "convertible": result.convertible,
        "valid": result.valid,
        "findings": all_findings,
        "total_constraints": result.total_constraints,
        "satisfied_constraints": result.satisfied_constraints,
        "unmodelled_constraints": result.unmodelled_constraints,
        "unchecked_constraints": result.unchecked_constraints,
        "historical_constraints": result.historical_constraints,
        "backend": result.backend,
    });
    if !result.constraints_summary.is_empty() {
        output["constraints"] = serde_json::json!(result.constraints_summary);
    }

    if let Some(reason) = &result.skip_reason {
        output["skip_reason"] = serde_json::json!(reason);
    }
    // `--verbose` adds supplementary info only; it never changes which core
    // findings are present (design D5).
    if verbose {
        if let Some(report) = &result.convertibility_report
            && !report.rephrase_directives.is_empty()
        {
            output["rephrase_directives"] = serde_json::json!(report.rephrase_directives);
        }
        if !result.constraints_summary.is_empty() {
            output["constraints_summary"] = serde_json::json!(result.constraints_summary);
        }
    }

    serde_json::to_string_pretty(&output).unwrap_or_default()
}
fn resolve_source(v: &Violation, plan: &PlanIR) -> (Option<String>, Option<String>) {
    let task_source = v.task_source.clone().or_else(|| {
        helpers::task_ids_from_ltl(&v.ltl).first().and_then(|id| {
            plan.tasks
                .iter()
                .find(|t| t.id == *id)
                .map(|t| format!("{}:{}", t.source.file, t.source.start_line))
        })
    });

    let req_source = v.req_source.clone().or_else(|| {
        plan.requirements
            .iter()
            .find(|r| r.id == v.constraint_id)
            .map(|r| format!("{}:{}", r.source.file, r.source.start_line))
    });

    (task_source, req_source)
}
fn verbose_section(
    output: &mut String,
    plans: &[(String, PlanIR)],
    _result: &crate::checker::VerificationResult,
) {
    for (name, plan) in plans {
        output.push_str(&format!("\n=== Plan: {} ===\n", name));
        output.push_str(&format!("Tasks: {}\n", plan.tasks.len()));
        output.push_str(&format!("Requirements: {}\n", plan.requirements.len()));
        output.push_str(&format!("Scenarios: {}\n", plan.scenarios.len()));
        output.push_str(&format!("Phases: {}\n", plan.phases.len()));
    }
}

#[cfg(test)]
mod tests;

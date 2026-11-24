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
        // No model formula backs a convertibility or prose finding. The flag describes the
        // model, so a prose blocker keeps `advisory: true` and still blocks the verdict.
        advisory: true,
        element: item.element.clone(),
        excerpt: item.excerpt.clone(),
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
        element: v.violation.constraint_id.clone(),
        excerpt: None,
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
/// Group findings that prescribe one remedy, keeping one true member of each group.
///
/// Grouping by `kind` alone treated two findings as interchangeable when they named different
/// sites, and then printed one member's location for the whole group — a substitution that
/// deleted the other members from the report. A group is now a summary of members that share
/// a remedy, and the members it does not print are counted, never described.
pub fn group_by_remedy(findings: &[Finding]) -> Vec<(usize, &Finding)> {
    let mut groups: Vec<(usize, &Finding)> = Vec::new();
    for f in findings {
        let remedy = f.suggestion.as_deref().unwrap_or(&f.message);
        let same = groups.iter_mut().find(|(_, g)| {
            g.kind == f.kind && g.suggestion.as_deref().unwrap_or(&g.message) == remedy
        });
        if let Some((count, _)) = same {
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

    // The run-level reason says the run was incomplete. This says which claim stopped, why, and
    // what it cost: "used up" does not tell a caller whether the claim came four thousand states
    // short or four million.
    let stopped: Vec<_> = result
        .constraints_summary
        .iter()
        .filter(|c| c.unchecked)
        .collect();
    if !stopped.is_empty() {
        output.push_str("\n  Unchecked claims:\n");
        for c in stopped {
            output.push_str(&format!(
                "    {}  {}",
                c.requirement_id,
                c.unchecked_reason
                    .as_deref()
                    .unwrap_or("no reason reported")
            ));
            if let (Some(used), Some(allowed)) = (c.states_used, c.states_allowed) {
                output.push_str(&format!(" (states {used} of {allowed})"));
            }
            output.push('\n');
        }
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
/// Render findings. A report holding one finding shows the whole identity; a report holding
/// several shows one line per kind and remedy (design D6). `--verbose` lists every finding.
fn format_findings(output: &mut String, all_findings: &[Finding], verbose: bool) {
    if verbose || all_findings.len() == 1 {
        for f in all_findings {
            format_one(output, f);
        }
        return;
    }
    for (count, f) in group_by_remedy(all_findings) {
        let remedy = f.suggestion.as_deref().unwrap_or(&f.message);
        let pointer = pointer_of(f);
        let mut line = format!("  {}× {}: {}", count, f.kind, remedy);
        if !pointer.is_empty() {
            line.push_str(&format!("  {pointer}"));
        }
        if count > 1 {
            line.push_str(&format!("  +{} more (--verbose)", count - 1));
        }
        output.push_str(&line);
        output.push('\n');
    }
}

/// One finding, with the element that owns the finding and the offending text.
fn format_one(output: &mut String, f: &Finding) {
    let pointer = pointer_of(f);
    if pointer.is_empty() {
        output.push_str(&format!("  [{}] {}\n", f.severity.to_uppercase(), f.kind));
    } else {
        output.push_str(&format!(
            "  [{}] {}  {}\n",
            f.severity.to_uppercase(),
            f.kind,
            pointer
        ));
    }
    if !f.element.is_empty() {
        output.push_str(&format!("    {}\n", f.element));
    }
    output.push_str(&format!("    {}\n", f.message));
    if let Some(excerpt) = &f.excerpt
        && !excerpt.trim().is_empty()
    {
        output.push_str(&format!("    \"{}\"\n", excerpt.trim()));
    }
    if let Some(s) = &f.suggestion {
        output.push_str(&format!("        Fix: {}\n", s));
    }
}

/// `path:line`, or empty when the finding carries no address. A plan-scoped kind has no
/// single site, so a pointer would name a place the finding is not about.
fn pointer_of(f: &Finding) -> String {
    if f.file.is_empty() || crate::ir::kind_of(&f.kind).arity() == crate::ir::Arity::Plan {
        return String::new();
    }
    if f.line == 0 {
        f.file.clone()
    } else {
        format!("{}:{}", f.file, f.line)
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

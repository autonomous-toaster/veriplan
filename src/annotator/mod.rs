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
        for item in report.blockers.iter().chain(report.warnings.iter()).chain(report.info.iter()) {
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
    if let Some((file, line)) = loc.rsplit_once(':') {
        if let Ok(line) = line.parse::<usize>() {
            return (file.to_string(), line);
        }
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

    let status = status_label(result);
    output.push_str(&format!("Plan: {} — {}\n", result.plan_name, status));

    if let Some(reason) = &result.skip_reason {
        output.push_str(&format!("\n  Model check skipped: {}\n", reason));
    }

    let all_findings = findings(result, annotated);
    if !all_findings.is_empty() {
        format_findings(&mut output, &all_findings, verbose);
    } else if result.convertible && result.valid == Some(true) {
        output.push_str("  All constraints satisfied.\n");
        output.push_str(&format!(
            "  Satisfied: {} | Violated: 0 | Total: {}\n",
            result.satisfied_constraints, result.total_constraints
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

fn status_label(result: &crate::checker::VerificationResult) -> &'static str {
    if result.convertible && result.valid == Some(true) {
        "✓ VALID"
    } else if !result.convertible {
        "⚠ SKIPPED"
    } else if result.valid == Some(false) {
        "✗ INVALID"
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
    });

    if let Some(reason) = &result.skip_reason {
        output["skip_reason"] = serde_json::json!(reason);
    }

    // `--verbose` adds supplementary info only; it never changes which core
    // findings are present (design D5).
    if verbose {
        if let Some(report) = &result.convertibility_report {
            if !report.rephrase_directives.is_empty() {
                output["rephrase_directives"] = serde_json::json!(report.rephrase_directives);
            }
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
        output.push_str(&format!("Phases: {}\n", plan.phases.len()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::{VerificationResult, Violation};
    use crate::ir::{ConvertibilityReport, PlanIR, SourceLocation, Task};

    fn make_plan() -> PlanIR {
        PlanIR {
            tasks: vec![Task {
                id: "1.1".into(),
                description: "Setup".into(),
                phase: "Phase 1".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 1,
                    end_line: 1,
                },
            }],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![],
            source_map: crate::ir::SourceMap::default(),
        }
    }

    fn make_result() -> VerificationResult {
        VerificationResult {
            plan_name: "test".into(),
            phase: "full".into(),
            convertible: true,
            convertibility_report: None,
            valid: Some(false),
            violations: vec![Violation {
                constraint_id: "R1".into(),
                requirement_statement: "T1.1 SHALL complete".into(),
                ltl: "[] ( active_t1_1 -> done_t1_1 )".into(),
                category: "SequentialOrder".into(),
                state: "".into(),
                task_source: None,
                req_source: None,
                suggested_fix: Some("Add before-task".into()),
                plan: "test".into(),
                kind: crate::ir::Kind::ViolationSequential,
                op: crate::ir::Op::ReplaceBody,
            }],
            total_constraints: 5,
            satisfied_constraints: 4,
            constraints_summary: vec![],
            skip_reason: None,
        }
    }

    fn make_plans() -> Vec<(String, PlanIR)> {
        vec![("test".into(), make_plan())]
    }

    #[test]
    fn test_format_human_contains_requirement() {
        let result = make_result();
        let plans = make_plans();
        let annotated = annotate(&result, &plans);
        let output = format_human(&result, &annotated, &plans, false);
        // Grouped human output shows the kind and the suggestion.
        assert!(output.contains("violation_sequential"));
        assert!(output.contains("Add before-task"));
    }

    #[test]
    fn test_findings_projection_flattens_violations() {
        let result = make_result();
        let plans = make_plans();
        let annotated = annotate(&result, &plans);
        let all = findings(&result, &annotated);
        assert!(!all.is_empty(), "expected at least one finding");
        assert!(
            all.iter().any(|f| f.kind == "violation_sequential"),
            "expected a violation_sequential finding: {:?}",
            all
        );
    }

    #[test]
    fn test_format_json_contains_findings() {
        let result = make_result();
        let plans = make_plans();
        let annotated = annotate(&result, &plans);
        let json = format_json(&result, &annotated, &plans, false);
        assert!(json.contains("\"findings\""));
        assert!(json.contains("\"R1\""));
        assert!(json.contains("\"suggestion\""));
        // The old top-level `violations` key is dropped (BREAKING).
        assert!(!json.contains("\"violations\""));
    }

    #[test]
    fn test_format_json_convertible() {
        let result = make_result();
        let plans = make_plans();
        let annotated = annotate(&result, &plans);
        let json = format_json(&result, &annotated, &plans, false);
        assert!(json.contains("\"convertible\": true"));
    }

    #[test]
    fn test_format_json_verbose_adds_supplementary_only() {
        let mut result = make_result();
        result.convertibility_report = Some(ConvertibilityReport {
            status: crate::ir::ConvertibilityStatus::Convertible,
            blockers: vec![],
            warnings: vec![],
            info: vec![],
            rephrase_directives: vec!["rephrase me".into()],
        });
        let plans = make_plans();
        let annotated = annotate(&result, &plans);
        let json = format_json(&result, &annotated, &plans, true);
        // `--verbose` adds rephrase_directives but never the old report key.
        assert!(json.contains("rephrase_directives"));
        assert!(!json.contains("convertibility_report"));
    }

    #[test]
    fn test_format_json_not_verbose_still_has_findings() {
        let mut result = make_result();
        result.convertibility_report = Some(ConvertibilityReport {
            status: crate::ir::ConvertibilityStatus::Convertible,
            blockers: vec![],
            warnings: vec![],
            info: vec![],
            rephrase_directives: vec![],
        });
        let plans = make_plans();
        let annotated = annotate(&result, &plans);
        let json = format_json(&result, &annotated, &plans, false);
        // Findings are always present at default verbosity (design D5).
        assert!(json.contains("\"findings\""));
        assert!(!json.contains("convertibility_report"));
    }

    #[test]
    fn test_group_by_kind_groups_identical() {
        let result = make_result();
        let plans = make_plans();
        let annotated = annotate(&result, &plans);
        let all = findings(&result, &annotated);
        let groups = group_by_kind(&all);
        // All findings here share one kind, so they collapse to one group.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, all.len());
    }

    #[test]
    fn test_json_and_human_describe_same_findings() {
        // Default JSON and default human output must describe the same set of
        // `Finding`s (design D5, task 8.3). The human output groups by kind,
        // so we compare the set of kinds + severities.
        let result = make_result();
        let plans = make_plans();
        let annotated = annotate(&result, &plans);

        let all = findings(&result, &annotated);
        let json = format_json(&result, &annotated, &plans, false);
        let human = format_human(&result, &annotated, &plans, false);

        // Every finding kind appears in both formats.
        for f in &all {
            assert!(
                json.contains(&format!("\"kind\": \"{}\"", f.kind)),
                "JSON missing kind {}: {}",
                f.kind,
                json
            );
            assert!(
                human.contains(&f.kind),
                "human output missing kind {}: {}",
                f.kind,
                human
            );
        }
    }
}

//! Tests for the annotator, kept out of mod.rs to stay under the file-length gate.
// Tests fail by panicking; the crate denies these three lints for code that ships.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::checker::{VerificationResult, Violation};
use crate::ir::{CheckItem, ConvertibilityReport, Fixability, Op, PlanIR, SourceLocation, Task};

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
        unmodelled_constraints: 0,
        unchecked_constraints: 0,
        historical_constraints: 0,
        backend: "test".into(),
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
#[test]
fn verdict_not_valid_when_blocker_finding_present() {
    // The verdict derives from the flattened findings, so a `blocker`
    // finding prevents "✓ VALID" even when the model-check result is valid
    // (guards the original "✓ VALID" + "[BLOCKER]" contradiction).
    let mut result = make_result();
    result.convertible = true;
    result.valid = Some(true);
    result.violations = vec![];
    result.convertibility_report = Some(ConvertibilityReport {
        status: crate::ir::ConvertibilityStatus::Convertible,
        blockers: vec![CheckItem {
            severity: "blocker".into(),
            check: "style/pronoun-ambiguity".into(),
            element: "Task 1.1".into(),
            location: "tasks.md:3".into(),
            detail: "pronoun 'it' is ambiguous".into(),
            fix: Some("replace the pronoun with the specific noun".into()),
            kind: crate::ir::Kind::ProseOther,
            op: Op::None,
            fixability: Fixability::Structural,
            start: 0,
            end: 0,
            replacement: None,
        }],
        warnings: vec![],
        info: vec![],
        rephrase_directives: vec![],
    });
    let plans = make_plans();
    let annotated = annotate(&result, &plans);
    let human = format_human(&result, &annotated, &plans, false);
    assert!(
        !human.contains("✓ VALID"),
        "verdict must not be VALID when a blocker finding is present: {}",
        human
    );
}

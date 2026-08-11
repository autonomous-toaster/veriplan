use super::*;
use crate::ir::{PlanIR, Requirement, Rfc2119Strength, SourceLocation, Task};

fn req(id: &str, statement: &str) -> Requirement {
    Requirement {
        id: id.to_string(),
        statement: statement.to_string(),
        strength: Rfc2119Strength::Must,
        category: crate::ir::ConstraintCategory::SequentialOrder,
        ltl: None,
        scenarios: vec![],
        source: SourceLocation {
            file: format!("{}.md", id),
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            end_line: 1,
        },
    }
}

fn task(id: &str, description: &str) -> Task {
    Task {
        id: id.to_string(),
        description: description.to_string(),
        phase: "Phase 1".into(),
        checked: false,
        source: SourceLocation {
            file: "tasks.md".into(),
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            end_line: 1,
        },
    }
}

fn plan_with(reqs: Vec<Requirement>, tasks: Vec<Task>) -> PlanIR {
    PlanIR {
        tasks,
        requirements: reqs,
        scenarios: vec![],
        phases: vec![],
        source_map: crate::ir::SourceMap::default(),
    }
}

#[test]
fn curated_rules_do_not_flag_openspec_grammar() {
    // "shall" (RFC 2119) and scenario scaffolding must not produce findings.
    let plan = plan_with(
        vec![req(
            "cap::ReqA",
            "T1.1 SHALL complete BEFORE T2.1 SHALL run.",
        )],
        vec![],
    );
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    // No dictionary / noun-cluster / topic-sentence findings for "shall",
    // "create", or GIVEN/WHEN/THEN scaffolding.
    for f in &findings {
        assert!(
            !f.rule.starts_with("dictionary/"),
            "unexpected dictionary finding: {:?}",
            f.message
        );
        assert!(
            f.rule != "structure/noun-cluster" && f.rule != "structure/topic-sentence",
            "unexpected structural finding: {:?}",
            f.message
        );
    }
}

#[test]
fn scenario_then_step_not_flagged_as_passive() {
    // A requirement statement that is active (has task agent) must not be
    // flagged passive — and scenario `**THEN**` steps are never in the
    // requirement statement (they are extracted separately).
    let plan = plan_with(
        vec![req(
            "cap::ReqA",
            "T1.2 SHALL resolve the file path BEFORE T1.5 dispatches to the parser.",
        )],
        vec![],
    );
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    assert!(
        !findings.iter().any(|f| f.rule == "verb/passive"),
        "active requirement should not be flagged passive: {:?}",
        findings
    );
}

#[test]
fn passive_requirement_is_flagged() {
    let plan = plan_with(
        vec![req(
            "cap::ReqA",
            "A .md file path SHALL be resolved relative to the current working directory.",
        )],
        vec![],
    );
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    assert!(
        findings.iter().any(|f| f.rule == "verb/passive"),
        "passive requirement should be flagged: {:?}",
        findings
    );
}

#[test]
fn tasks_get_minimal_rule_subset_only() {
    // tasks.md should only get OneInstructionPerSentence + Hedging, not passive.
    let plan = plan_with(
        vec![],
        vec![task(
            "1.1",
            "Open the crate and read the docs and verify the version.",
        )],
    );
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    // The one-instruction rule may fire; passive must NOT fire on task desc.
    assert!(
        !findings.iter().any(|f| f.rule == "verb/passive"),
        "tasks.md should not get passive rule: {:?}",
        findings
    );
}

#[test]
fn strictness_mapping_downgrades_in_lax() {
    let plan = plan_with(
        vec![req(
            "cap::ReqA",
            "A .md file path SHALL be resolved relative to the current working directory.",
        )],
        vec![],
    );
    let strict = check_prose(&plan, None, &StrictnessProfile::Strict);
    let lax = check_prose(&plan, None, &StrictnessProfile::Lax);
    // In strict, passive is a blocker; in lax it's info.
    assert!(
        strict
            .iter()
            .any(|f| f.rule == "verb/passive" && f.severity == "blocker"),
        "strict should mark passive as blocker: {:?}",
        strict
    );
    assert!(
        lax.iter().any(|f| f.rule == "verb/passive" && f.severity == "info"),
        "lax should downgrade passive to info: {:?}",
        lax
    );
}

#[test]
fn combined_directive_when_passive_and_ungrounded() {
    // A passive requirement with no task agent grounds poorly.
    let plan = plan_with(
        vec![req(
            "cap::ReqA",
            "A .md file path SHALL be resolved relative to the current working directory.",
        )],
        vec![task("1.1", "Resolve paths")],
    );
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    let directives = correlate_with_grounding(&findings, &plan, &StrictnessProfile::Strict);
    // Find the directive for the passive requirement.
    let passive_findings: Vec<&ProseFinding> =
        findings.iter().filter(|f| f.rule == "verb/passive").collect();
    assert!(!passive_findings.is_empty(), "expected a passive finding");
    // correlation should produce a combined directive (or the requirement is
    // not ungroundable if it references a task — here it does not).
    assert!(
        !directives.is_empty(),
        "expected a combined directive for passive+ungrounded requirement"
    );
    let combined = &directives[0];
    assert!(
        combined.directive.contains("agent"),
        "directive should mention naming the agent task ID: {}",
        combined.directive
    );
}

#[test]
fn no_combined_directive_when_active_and_grounded() {
    // An active requirement with explicit task agent should ground cleanly.
    let plan = plan_with(
        vec![req(
            "cap::ReqA",
            "T1.1 SHALL resolve paths BEFORE T1.2 SHALL dispatch.",
        )],
        vec![task("1.1", "Resolve paths"), task("1.2", "Dispatch")],
    );
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    let directives = correlate_with_grounding(&findings, &plan, &StrictnessProfile::Strict);
    assert!(
        directives.is_empty(),
        "active+grounded requirement should not get a combined directive: {:?}",
        directives
    );
}

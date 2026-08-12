use super::*;
use crate::ir::{PlanIR, Requirement, Rfc2119Strength, Scenario, ScenarioStep, SourceLocation, StepKind, Task};

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

fn scenario_step(kind: StepKind, text: &str) -> ScenarioStep {
    ScenarioStep {
        kind,
        text: text.to_string(),
        source: SourceLocation {
            file: "spec.md".into(),
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            end_line: 1,
        },
    }
}

fn plan_with_scenario(steps: Vec<ScenarioStep>) -> PlanIR {
    let mut plan = plan_with(
        vec![req(
            "cap::ReqA",
            "T1.1 SHALL complete BEFORE T1.2 SHALL start.",
        )],
        vec![],
    );
    plan.scenarios = vec![Scenario {
        name: "Test".into(),
        steps,
        source: SourceLocation {
            file: "spec.md".into(),
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            end_line: 1,
        },
    }];
    plan
}

#[test]
fn scenario_legitimate_state_assertion_not_flagged_passive() {
    // A scenario THEN step describing a state ("the plan SHALL be marked
    // VALID") must NOT get a PassiveVoice or OneInstructionPerSentence finding
    // — the safe subset excludes those rules (spec R1.2, R1.3).
    let plan = plan_with_scenario(vec![scenario_step(
        StepKind::Then,
        "**THEN** the plan SHALL be marked VALID",
    )]);
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    assert!(
        !findings.iter().any(|f| f.rule == "verb/passive"),
        "legitimate state assertion must not be flagged passive: {:?}",
        findings
    );
    assert!(
        !findings
            .iter()
            .any(|f| f.rule == "structure/one-instruction-per-sentence"),
        "legitimate state assertion must not be flagged one-instruction: {:?}",
        findings
    );
}

#[test]
fn scenario_ambiguous_pronoun_is_flagged() {
    // A real scenario step from the steve corpus with two real noun
    // antecedents ("Bazel", "steve library") and an ambiguous pronoun ("its")
    // must be flagged by PronounAmbiguity (spec R1.1).
    let plan = plan_with_scenario(vec![scenario_step(
        StepKind::Then,
        "**THEN** Bazel compiles the steve library and its dependencies hermetically",
    )]);
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    assert!(
        findings.iter().any(|f| f.rule == "style/pronoun-ambiguity"),
        "ambiguous pronoun in real scenario step should be flagged: {:?}",
        findings
    );
}

#[test]
fn real_corpus_clear_scenarios_not_flagged() {
    // Real scenario steps from the sibling projects that use a pronoun with a
    // clear referent must NOT be flagged (no false positives). This is the
    // key regression guard for the steve pronoun fix.
    let clear_steps = [
        "it returns a Report for that text",
        "the engine does not flag it as passive voice",
        "it carries the same fields as the CLI json output",
        "it reads and checks standard input",
        "maturin builds it via Cargo",
    ];
    let mut b = Ste::builder();
    for r in crate::prose::all_rules() {
        b = b.rule(r, steve::Severity::Off);
    }
    b = b
        .rule(steve::RuleId::PronounAmbiguity, steve::Severity::Soft)
        .rule(steve::RuleId::SentenceLength, steve::Severity::Soft);
    let ste = b.build().unwrap();
    for s in clear_steps {
        let report = ste.check_text(s).unwrap();
        assert!(
            report.is_empty(),
            "clear real scenario step should not be flagged: {:?} => {:?}",
            s,
            report.iter().map(|f| f.message().to_string()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn scenario_scaffolding_stripped_before_checking() {
    // The **THEN** marker and inline code spans must be stripped before
    // checking, so they don't produce structural noise (spec R1.2).
    let stripped = strip_scenario_step("**THEN** the engine does not flag `check` or `verify`");
    // Code spans are removed entirely (their content is not prose).
    assert_eq!(stripped, "the engine does not flag  or");
    let stripped2 = strip_scenario_step("**GIVEN** the system is initialized");
    assert_eq!(stripped2, "the system is initialized");
    // The **THEN** marker is removed.
    assert!(!stripped.starts_with("**THEN**"));
}

#[test]
fn scenario_prose_findings_advisory_not_blocking() {
    // Scenario prose findings are advisory (warnings/info), never blockers —
    // the plan stays convertible (spec R2.1). check_prose returns findings
    // that the checker maps to warnings/info, never a blocker.
    let plan = plan_with_scenario(vec![scenario_step(
        StepKind::Then,
        "**THEN** the valve and the pump are connected, and it is faulty",
    )]);
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    for f in &findings {
        assert_ne!(f.severity, "blocker", "scenario prose must not block: {:?}", f);
    }
}

#[test]
fn scenario_verdict_unchanged() {
    // The presence of scenario prose findings must not change the plan verdict
    // (no requirement is newly blocked). check_prose only returns advisory
    // findings; the convertibility verdict is unaffected.
    let plan = plan_with_scenario(vec![scenario_step(
        StepKind::Then,
        "**THEN** the valve and the pump are connected, and it is faulty",
    )]);
    let findings = check_prose(&plan, None, &StrictnessProfile::Strict);
    // The requirement is still a valid temporal constraint.
    let cat = crate::translator::classify(&plan.requirements[0].statement);
    // Temporal category (may be FixedTime or SequentialOrder) — not NonFormalizable.
    assert_ne!(cat, crate::ir::ConstraintCategory::NonFormalizable);
    assert!(!findings.is_empty(), "scenario finding expected");
}

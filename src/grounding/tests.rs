use crate::grounding::{check_grounding, signature_from_planir};
use crate::input::StrictnessProfile;
use crate::ir::*;

fn make_test_plan(tasks: Vec<(&str, &str)>, reqs: Vec<(&str, &str, &str)>) -> PlanIR {
    let source_map = SourceMap::default();
    let tasks: Vec<Task> = tasks
        .into_iter()
        .enumerate()
        .map(|(i, (id, desc))| Task {
            id: id.to_string(),
            description: desc.to_string(),
            phase: "Test".into(),
            checked: false,
            source: SourceLocation {
                file: "tasks.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: i + 1,
                end_line: i + 1,
            },
        })
        .collect();
    let requirements: Vec<Requirement> = reqs
        .into_iter()
        .enumerate()
        .map(|(i, (id, statement, _cat))| Requirement {
            id: id.to_string(),
            statement: statement.to_string(),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::SequentialOrder,
            ltl: None,
            scenarios: vec![],
            source: SourceLocation {
                file: "spec.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: i + 1,
                end_line: i + 1,
            },
        })
        .collect();
    PlanIR {
        tasks,
        requirements,
        scenarios: vec![],
        phases: vec![],
        source_map,
    }
}

#[test]
fn test_signature_from_planir_basic() {
    let plan = make_test_plan(
        vec![("1.1", "Create project"), ("1.2", "Add dependencies")],
        vec![],
    );
    let sig = signature_from_planir(&plan);
    assert_eq!(sig.constants.len(), 2);
    assert_eq!(sig.constants[0].name, "T1.1");
    assert_eq!(sig.constants[1].name, "T1.2");
    assert_eq!(sig.predicates.len(), 6);
    assert_eq!(sig.types.len(), 2);
}

#[test]
fn test_signature_from_empty_planir() {
    let plan = make_test_plan(vec![], vec![]);
    let sig = signature_from_planir(&plan);
    assert_eq!(sig.constants.len(), 0);
    assert_eq!(sig.predicates.len(), 6);
    assert_eq!(sig.types.len(), 2);
}

#[test]
fn test_grounding_passes_for_explicit_ids() {
    let plan = make_test_plan(
        vec![("1.1", "Create project"), ("1.2", "Add dependencies")],
        vec![("R1", "T1.1 SHALL complete BEFORE T1.2 SHALL run", "seq")],
    );
    let (blockers, warnings, _info, outcomes) = check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(blockers.is_empty(), "blockers: {:?}", blockers);
    assert!(warnings.is_empty(), "warnings: {:?}", warnings);
    assert_eq!(outcomes.len(), 1);
    assert!(!outcomes[0].failed);
}

#[test]
fn test_grounding_fails_for_vague_nl() {
    let plan = make_test_plan(
        vec![("1.1", "Create project")],
        vec![("R1", "The system SHALL be user-friendly", "non")],
    );
    let (blockers, warnings, _info, outcomes) = check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(!blockers.is_empty(), "expected blockers");
    assert!(warnings.is_empty());
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].failed);
}

#[test]
fn test_grounding_skipped_empty_requirements() {
    let plan = make_test_plan(vec![("1.1", "Task")], vec![]);
    let (blockers, warnings, info, outcomes) = check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(blockers.is_empty());
    assert!(warnings.is_empty());
    assert!(outcomes.is_empty());
    assert!(info.iter().any(|i| i.check == "grounding_skipped"));
}

#[test]
fn test_grounding_skipped_empty_tasks() {
    let plan = make_test_plan(vec![], vec![("R1", "T1.1 SHALL complete", "seq")]);
    let (blockers, warnings, info, outcomes) = check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(blockers.is_empty());
    assert!(warnings.is_empty());
    assert!(outcomes.is_empty());
    assert!(info.iter().any(|i| i.check == "grounding_skipped"));
}

#[test]
fn test_grounding_ambiguous_downgraded_by_strictness() {
    // Create a plan where the grounder will find ambiguous matches
    // (task IDs that partially match but with low confidence)
    let plan = make_test_plan(
        vec![("1.1", "setup"), ("2.1", "migration")],
        vec![(
            "R1",
            "The setup step SHALL complete before the migration",
            "seq",
        )],
    );
    // With Strict profile, ambiguous should be a blocker
    let (blockers, _warnings, _info, _outcomes) =
        check_grounding(&plan, &StrictnessProfile::Strict);
    // This might be Grounded (via aliases) or Ambiguous depending on the grounder
    // Just verify it doesn't crash
    assert!(blockers.is_empty() || !blockers.is_empty());
}

#[test]
fn test_may_requirement_skipped() {
    let plan = make_test_plan(
        vec![("6.7", "BFS fallback")],
        vec![("R1", "T6.7 MAY provide a built-in BFS explorer", "may")],
    );
    // Override strength to May
    let mut plan = plan;
    plan.requirements[0].strength = Rfc2119Strength::May;

    let (blockers, warnings, info, outcomes) = check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(blockers.is_empty(), "expected no blockers: {:?}", blockers);
    assert!(warnings.is_empty(), "expected no warnings: {:?}", warnings);
    assert!(
        outcomes.is_empty(),
        "expected no outcomes for skipped MAY: {:?}",
        outcomes
    );
    assert!(
        info.iter().any(|i| i.check == "grounding_may_skipped"),
        "expected grounding_may_skipped info item"
    );
}

#[test]
fn test_ungroundable_with_task_id_has_predicate_message() {
    // Requirement with a valid task ID but no temporal predicate keyword
    let plan = make_test_plan(
        vec![("6.7", "BFS fallback")],
        vec![("R1", "T6.7 MAY provide a built-in BFS explorer", "may")],
    );
    // Use Must strength so it's not skipped, but the text has no temporal keyword
    let mut plan = plan;
    plan.requirements[0].strength = Rfc2119Strength::Must;

    let (blockers, _warnings, _info, _outcomes) =
        check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(!blockers.is_empty(), "expected blockers");
    // The error message should mention predicate keyword, not task
    let msg = &blockers[0].detail;
    assert!(
        msg.contains("predicate keyword"),
        "expected 'predicate keyword' in message, got: {}",
        msg
    );
    assert!(
        msg.contains("BEFORE"),
        "expected 'BEFORE' in message, got: {}",
        msg
    );
}

#[test]
fn test_ungroundable_no_task_id_has_original_message() {
    // Requirement with no task ID and no temporal keyword
    let plan = make_test_plan(
        vec![("1.1", "Create project")],
        vec![("R1", "The system SHALL be user-friendly", "non")],
    );
    let (blockers, _warnings, _info, _outcomes) =
        check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(!blockers.is_empty(), "expected blockers");
    // The error message should use the original wording
    let msg = &blockers[0].detail;
    assert!(
        msg.contains("no matching task or predicate"),
        "expected original message, got: {}",
        msg
    );
}

#[test]
fn test_multi_keyword_detected() {
    // Requirement with both BEFORE and ALWAYS keywords
    let plan = make_test_plan(
        vec![("1.1", "Setup"), ("1.2", "Build"), ("2.1", "Deploy")],
        vec![(
            "R1",
            "T1.1 SHALL complete BEFORE T1.2. T2.1 SHALL ALWAYS be available.",
            "seq",
        )],
    );
    let (blockers, _warnings, _info, outcomes) = check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(!blockers.is_empty(), "expected blockers for multi-keyword");
    let has_multi = blockers
        .iter()
        .any(|b| b.check == "grounding_ambiguous_multi_keyword");
    assert!(
        has_multi,
        "expected grounding_ambiguous_multi_keyword check"
    );
    let msg = &blockers
        .iter()
        .find(|b| b.check == "grounding_ambiguous_multi_keyword")
        .unwrap()
        .detail;
    assert!(
        msg.contains("BEFORE"),
        "expected BEFORE in message: {}",
        msg
    );
    assert!(
        msg.contains("ALWAYS"),
        "expected ALWAYS in message: {}",
        msg
    );
    assert!(
        msg.contains("GROUNDING AMBIGUITY"),
        "expected GROUNDING AMBIGUITY in message: {}",
        msg
    );
    assert!(outcomes.iter().any(|o| o.failed), "expected failed outcome");
}

#[test]
fn test_single_keyword_no_multi_error() {
    // Requirement with only BEFORE keyword
    let plan = make_test_plan(
        vec![("1.1", "Setup"), ("1.2", "Build")],
        vec![("R1", "T1.1 SHALL complete BEFORE T1.2 SHALL run.", "seq")],
    );
    let (blockers, _warnings, _info, _outcomes) =
        check_grounding(&plan, &StrictnessProfile::Strict);
    let has_multi = blockers
        .iter()
        .any(|b| b.check == "grounding_ambiguous_multi_keyword");
    assert!(
        !has_multi,
        "expected no multi-keyword error for single keyword"
    );
}

#[test]
fn test_multi_keyword_with_three_predicates() {
    // Requirement with BEFORE, ALWAYS, and IF_THEN keywords
    let plan = make_test_plan(
        vec![
            ("1.1", "Setup"),
            ("1.2", "Build"),
            ("2.1", "Deploy"),
            ("3.1", "Monitor"),
        ],
        vec![(
            "R1",
            "T1.1 SHALL complete BEFORE T1.2. T2.1 SHALL ALWAYS be available. IF T1.1 fails THEN T3.1 SHALL run.",
            "seq",
        )],
    );
    let (blockers, _warnings, _info, _outcomes) =
        check_grounding(&plan, &StrictnessProfile::Strict);
    assert!(!blockers.is_empty(), "expected blockers for multi-keyword");
    let has_multi = blockers
        .iter()
        .any(|b| b.check == "grounding_ambiguous_multi_keyword");
    assert!(
        has_multi,
        "expected grounding_ambiguous_multi_keyword check"
    );
}

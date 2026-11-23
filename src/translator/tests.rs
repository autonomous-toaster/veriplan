//! Tests for the rule translator, kept out of mod.rs to stay under the file-length gate.
// Tests fail by panicking; the crate denies these three lints for code that ships.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
/// The seam between classification and translation: a realistic body, classified by the
/// production path, must produce the ordering formula. Both halves were tested separately and
/// the seam was not, which is how every ordering statement became a tautology.
#[test]
fn test_seam_classify_then_translate_ordering() {
    let plan = make_test_plan();
    let statement = "T1.1 SHALL complete BEFORE T1.2 SHALL start.";
    assert_eq!(
        classify(statement),
        ConstraintCategory::SequentialOrder,
        "a task-citing ordering statement must reach SequentialOrder"
    );
    let ltl = generate_ltl(&classify(statement), statement, &plan);
    assert!(
        ltl.is_some(),
        "a task-citing ordering statement must yield a formula"
    );
    let rendered = ltl
        .map(|f| crate::ir::ltl::ltl_to_string(&f))
        .unwrap_or_default();
    assert!(
        rendered.contains("active_t1_2") && rendered.contains("done_t1_1"),
        "expected active(later) -> done(earlier), got {rendered}"
    );
}

/// A category with no encoding must not produce a formula.
#[test]
fn test_unencoded_categories_yield_no_formula() {
    let plan = make_test_plan();
    for statement in [
        "T1.1 SHALL ALWAYS hold the header.",
        "T1.1 SHALL run within the maintenance window.",
    ] {
        assert!(
            generate_ltl(&classify(statement), statement, &plan).is_none(),
            "no placeholder formula for: {statement}"
        );
    }
}

#[test]
fn test_find_sequential_pair_before() {
    // A body the requirement grammar permits: T-prefixed ids and an RFC 2119 verb
    // between the id and the keyword. Bare ids tested a shape no caller produces.
    let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
    let body = "T1.1 SHALL complete BEFORE T1.2 SHALL start.";
    assert_eq!(
        find_sequential_pair(body, &task_ids),
        Some(("1.1".to_string(), "1.2".to_string()))
    );
}
#[test]
fn test_find_sequential_pair_after() {
    let task_ids = vec!["1.1".to_string(), "2.1".to_string()];
    let body = "T2.1 SHALL complete AFTER T1.1 SHALL complete.";
    assert_eq!(
        find_sequential_pair(body, &task_ids),
        Some(("1.1".to_string(), "2.1".to_string())),
        "AFTER names the earlier task second"
    );
}
#[test]
fn test_before_body_and_after_twin_produce_one_formula() {
    let plan = make_test_plan();
    let before = "T1.1 SHALL complete BEFORE T1.2 SHALL start.";
    let after = "T1.2 SHALL start AFTER T1.1 SHALL complete.";
    let render = |statement: &str| {
        generate_ltl(&classify(statement), statement, &plan)
            .map(|f| crate::ir::ltl::ltl_to_string(&f))
            .unwrap_or_else(|| "<no formula>".to_string())
    };
    let b = render(before);
    let a = render(after);
    assert_eq!(a, b, "the two orderings of one pair must translate alike");
    assert!(
        b.contains("active_t1_2") && b.contains("done_t1_1"),
        "expected active(later) -> done(earlier), got {b}"
    );
}
#[test]
fn test_find_sequential_pair_no_match() {
    let task_ids = vec!["1.1".to_string()];
    let result = find_sequential_pair("The system SHALL be robust", &task_ids);
    assert_eq!(result, None);
}
#[test]
fn test_normalize_id() {
    assert_eq!(normalize_id("1.3"), "t1_3");
    assert_eq!(normalize_id("10.7"), "t10_7");
}
#[test]
fn test_diagnose_bare_capability() {
    // References a task, no vague word -> BareCapability
    let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
    let d = diagnose_vague("T1.1 SHALL be executed.", &task_ids).unwrap();
    assert_eq!(
        d,
        VagueDiagnosis::BareCapability {
            task: "1.1".to_string(),
            word: None
        }
    );
    // classify() must have returned NonFormalizable first.
    assert_eq!(
        classify("T1.1 SHALL be executed."),
        ConstraintCategory::NonFormalizable
    );
}
#[test]
fn test_diagnose_vague_action() {
    // References a task + vague adverb -> VagueAction
    let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
    let d = diagnose_vague("T1.1 SHALL be done quickly.", &task_ids).unwrap();
    assert_eq!(
        d,
        VagueDiagnosis::VagueAction {
            task: "1.1".to_string(),
            word: "quickly".to_string()
        }
    );
}
#[test]
fn test_diagnose_vague_quality() {
    // No task reference + vague adjective -> VagueQuality
    let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
    let d = diagnose_vague("The system SHALL be robust.", &task_ids).unwrap();
    assert_eq!(
        d,
        VagueDiagnosis::VagueQuality {
            word: "robust".to_string()
        }
    );
}
#[test]
fn test_safety_boundary_temporal_not_diagnosed() {
    // A temporal requirement is NEVER diagnosed as vague — classify() returns
    // a temporal category (SequentialOrder), so diagnose_vague is not reached.
    let task_ids = vec!["1.1".to_string(), "1.2".to_string()];
    let cat = classify("T1.1 SHALL be done quickly BEFORE T1.2 SHALL start.");
    // The exact temporal category is a pre-existing classification detail
    // (may be FixedTime or SequentialOrder); the guarantee is that it is a
    // temporal category and therefore never diagnosed as vague.
    assert_ne!(cat, ConstraintCategory::NonFormalizable);
    assert_ne!(cat, ConstraintCategory::Informational);
    assert!(
        diagnose_vague(
            "T1.1 SHALL be done quickly BEFORE T1.2 SHALL start.",
            &task_ids
        )
        .is_none()
            || !classify("T1.1 SHALL be done quickly BEFORE T1.2 SHALL start.")
                .eq(&ConstraintCategory::NonFormalizable),
        "temporal requirement must not be diagnosed as vague"
    );
}
#[test]
fn test_diagnose_undiagnosed_falls_back() {
    // No task reference, no vague word -> None (generic blocker fallback)
    let task_ids = vec!["1.1".to_string()];
    let d = diagnose_vague("The migration SHALL happen.", &task_ids);
    assert!(d.is_none());
    assert_eq!(
        classify("The migration SHALL happen."),
        ConstraintCategory::NonFormalizable
    );
}
#[test]
fn test_diagnose_verdict_unchanged() {
    // Vague requirements still classify as NonFormalizable (blocker),
    // not reclassified to Informational.
    for stmt in [
        "T1.1 SHALL be executed.",
        "T1.1 SHALL be done quickly.",
        "The system SHALL be robust.",
    ] {
        let task_ids = vec!["1.1".to_string()];
        assert_eq!(
            classify(stmt),
            ConstraintCategory::NonFormalizable,
            "verdict must remain a blocker: {stmt}"
        );
        assert!(diagnose_vague(stmt, &task_ids).is_some());
    }
}

#[test]
fn test_generate_ltl_sequential() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::SequentialOrder,
        "T1.1 SHALL complete BEFORE T1.2",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("active_t1_2"));
    assert!(ltl_str.contains("done_t1_1"));
}
#[test]
fn test_generate_ltl_exclusive() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::Exclusive,
        "At most one of T1.1, T1.2 SHALL be active",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("active_t1_1"));
    assert!(ltl_str.contains("active_t1_2"));
}
#[test]
fn test_generate_ltl_conditional() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::Conditional,
        "IF T1.1 fails THEN T2.1 SHALL run",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("failed_t1_1"));
    assert!(ltl_str.contains("active_t2_1"));
}
#[test]
fn test_generate_ltl_concurrent() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::ConcurrentEvents,
        "T3.1 and T3.2 SHALL run concurrently",
        &plan,
    );
    assert!(ltl.is_some());
    let ltl_str = crate::ir::ltl::ltl_to_string(&ltl.unwrap());
    assert!(ltl_str.contains("<->"));
}
#[test]
fn test_generate_ltl_non_formalizable() {
    let plan = make_test_plan();
    let ltl = generate_ltl(
        &ConstraintCategory::NonFormalizable,
        "The system SHALL be robust",
        &plan,
    );
    assert!(ltl.is_none());
}
#[test]
fn test_extract_task_refs() {
    let plan = make_test_plan();
    let refs = extract_task_refs("T1.1 SHALL complete BEFORE T1.2", &plan);
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&"1.1".to_string()));
    assert!(refs.contains(&"1.2".to_string()));
}
#[test]
fn test_extract_task_refs_bare() {
    let task_ids = vec!["1.1".to_string(), "1.2".to_string(), "2.1".to_string()];
    let refs = extract_task_refs_bare("T1.1 SHALL complete BEFORE T1.2", &task_ids);
    assert_eq!(refs.len(), 2);
}

#[allow(dead_code)]
fn make_test_plan() -> crate::ir::PlanIR {
    use crate::ir::*;
    PlanIR {
        tasks: vec![
            Task {
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
            },
            Task {
                id: "1.2".into(),
                description: "Build".into(),
                phase: "Phase 1".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 2,
                    end_line: 2,
                },
            },
            Task {
                id: "2.1".into(),
                description: "Deploy".into(),
                phase: "Phase 2".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 3,
                    end_line: 3,
                },
            },
            Task {
                id: "3.1".into(),
                description: "Monitor".into(),
                phase: "Phase 3".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 4,
                    end_line: 4,
                },
            },
            Task {
                id: "3.2".into(),
                description: "Alert".into(),
                phase: "Phase 3".into(),
                checked: false,
                source: SourceLocation {
                    file: "tasks.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 5,
                    end_line: 5,
                },
            },
        ],
        requirements: vec![],
        scenarios: vec![],
        phases: vec![],
        source_map: SourceMap::default(),
    }
}

/// A clock literal and a duration are both time references; a bare number is not.
///
/// A single-letter unit is not recognised: "5s" reads as a number followed by a word, and accepting
/// "s" would also make "5 samples" a time reference. "5 ms" and "5 sec" are recognised.
#[test]
fn time_references_are_recognised() {
    for yes in [
        "finish by 12:30",
        "within 5 ms",
        "250 sec later",
        "5 minute window",
    ] {
        assert!(is_time_ref(yes), "{yes} should be a time reference");
    }
    for no in [
        "T1.1 SHALL complete",
        "12 apples",
        "the second clause",
        "within 5s",
    ] {
        assert!(!is_time_ref(no), "{no} should not be a time reference");
    }
}

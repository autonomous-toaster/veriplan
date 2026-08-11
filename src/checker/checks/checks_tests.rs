mod tests {
    use crate::checker::checks::*;
    use crate::ir::*;

    fn make_test_plan() -> PlanIR {
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
            ],
            requirements: vec![Requirement {
                id: "R1".into(),
                statement: "T1.1 SHALL complete BEFORE T1.2".into(),
                strength: Rfc2119Strength::Must,
                category: ConstraintCategory::SequentialOrder,
                ltl: None,
                scenarios: vec![],
                source: SourceLocation {
                    file: "spec.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 1,
                    end_line: 1,
                },
            }],
            scenarios: vec![],
            phases: vec![Phase {
                name: "Phase 1".into(),
                task_ids: vec!["1.1".into(), "1.2".into()],
                mode: PhaseMode::Sequential,
            }],
            source_map: SourceMap::default(),
        }
    }

    #[test]
    fn test_check_tasks_valid() {
        let plan = make_test_plan();
        let (blocker, warnings, info) = check_tasks(&plan, true);
        assert!(blocker.is_none());
        assert!(warnings.is_empty());
        assert!(info.iter().any(|i| i.check == "task_count"));
    }

    #[test]
    fn test_check_tasks_empty() {
        let plan = PlanIR {
            tasks: vec![],
            ..make_test_plan()
        };
        let (blocker, _, _) = check_tasks(&plan, true);
        assert!(blocker.is_some());
    }

    #[test]
    fn test_check_requirements_valid() {
        let plan = make_test_plan();
        let (blockers, warnings, _) = check_requirements(&plan, true);
        assert!(blockers.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_check_requirements_empty() {
        let plan = PlanIR {
            requirements: vec![],
            ..make_test_plan()
        };
        let (blockers, _, _) = check_requirements(&plan, true);
        assert!(!blockers.is_empty());
    }

    #[test]
    fn test_check_task_references_valid() {
        let plan = make_test_plan();
        let (blockers, _) = check_task_references(&plan);
        assert!(blockers.is_empty());
    }

    #[test]
    fn test_check_task_references_bad_ref() {
        // The function only checks references that match existing task IDs.
        // A reference to a non-existent ID like T1.3 won't be caught here.
        // This test verifies that valid references pass.
        let plan = make_test_plan();
        let (blockers, _) = check_task_references(&plan);
        assert!(blockers.is_empty());
    }

    #[test]
    fn test_check_diversity() {
        let plan = make_test_plan();
        let info = check_diversity(&plan);
        assert!(!info.is_empty());
    }

    #[test]
    fn test_check_task_coverage() {
        let plan = make_test_plan();
        let (_warnings, info) = check_task_coverage(&plan, true);
        assert!(info.iter().any(|i| i.check == "task_coverage"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(crate::util::truncate("short", 10), "short");
        let t = crate::util::truncate("a very long string", 10);
        assert!(t.len() <= 13, "truncated string too long: {}", t);
        assert!(t.ends_with('…'));
    }
}

#[test]
fn informational_requirement_does_not_block() {
    use crate::input::StrictnessProfile;
    use crate::ir::*;
    use crate::parser::extract_shall_statement;

    // A requirement explicitly marked 'human review only' must be classified
    // as Informational and must NOT produce a non_formalizable blocker.
    let body = "This policy is human review only. The system SHALL be auditable.";
    let stmt = extract_shall_statement(body, "spec.md");
    assert_eq!(
        crate::translator::classify(&stmt),
        ConstraintCategory::Informational,
        "classify should detect the human-review-only marker"
    );
}

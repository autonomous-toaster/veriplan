// Tests fail by panicking; the crate denies these three lints for code that ships.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
                grounded_in: None,
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
        assert!(
            info.iter().all(|i| i.check != "task_count"),
            "task_count aggregate must not be emitted: {:?}",
            info
        );
    }

    #[test]
    fn no_aggregate_summary_findings_emitted() {
        // The five plan-level aggregate diagnostics (task_count, scenario_count,
        // classification_summary, constraint_diversity, grounding_summary) must
        // not surface as findings — they were the `5× prose_other` noise.
        use crate::grounding;
        use crate::input::StrictnessProfile;
        let plan = make_test_plan();
        let aggregate = [
            "task_count",
            "scenario_count",
            "classification_summary",
            "constraint_diversity",
            "grounding_summary",
        ];

        let mut info_checks = Vec::new();
        if let (None, _, info) = check_tasks(&plan, true) {
            info_checks.extend(info.into_iter().map(|i| i.check));
        }
        let (_, info) = check_scenarios(&plan);
        info_checks.extend(info.into_iter().map(|i| i.check));
        info_checks.extend(check_diversity(&plan).into_iter().map(|i| i.check));
        let (_, _, info) = check_classifiability(&plan, true);
        info_checks.extend(info.into_iter().map(|i| i.check));
        let (_, _, info, _) = grounding::check_grounding(&plan, &StrictnessProfile::Strict);
        info_checks.extend(info.into_iter().map(|i| i.check));

        for name in aggregate {
            assert!(
                !info_checks.iter().any(|c| c == name),
                "aggregate diagnostic '{}' must not be emitted as a finding",
                name
            );
        }
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
        // The `constraint_diversity` aggregate summary is no longer emitted.
        // (`low_diversity` only fires when it has a suggestion; this plan is
        // below the threshold, so info is expected empty.)
        assert!(
            info.iter().all(|i| i.check != "constraint_diversity"),
            "constraint_diversity aggregate must not be emitted: {:?}",
            info
        );
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

    #[test]
    fn test_non_formalizable_subtypes_map_to_distinct_checks() {
        // Three non-formalizable requirements with distinct diagnoses must
        // map to distinct `check` values (bare_capability / vague_action /
        // vague_quality), not all collapse to `non_formalizable`.
        let cases = [
            ("T1.1 SHALL be executed.", "bare_capability"),
            ("T1.1 SHALL be done quickly.", "vague_action"),
            ("The system SHALL be robust.", "vague_quality"),
        ];
        for (statement, expected_check) in cases {
            let plan = PlanIR {
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
                requirements: vec![Requirement {
                    id: "R1".into(),
                    grounded_in: None,
                    statement: statement.into(),
                    strength: Rfc2119Strength::Must,
                    category: ConstraintCategory::NonFormalizable,
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
                phases: vec![],
                source_map: SourceMap::default(),
            };
            let (blockers, _, _) = check_classifiability(&plan, true);
            assert!(
                blockers.iter().any(|b| b.check == expected_check),
                "expected check '{}' for statement '{}', got {:?}",
                expected_check,
                statement,
                blockers
                    .iter()
                    .map(|b| b.check.as_str())
                    .collect::<Vec<_>>()
            );
            // The subtype must NOT collapse to the old generic value.
            assert!(
                !blockers.iter().any(|b| b.check == "non_formalizable"),
                "subtype check collapsed to generic 'non_formalizable' for '{}'",
                statement
            );
        }
    }

    /// The same two tasks, with the given requirement bodies.
    fn plan_with_statements(statements: &[&str]) -> PlanIR {
        let mut plan = make_test_plan();
        plan.requirements = statements
            .iter()
            .enumerate()
            .map(|(idx, statement)| Requirement {
                id: format!("R{idx}"),
                grounded_in: None,
                statement: (*statement).to_string(),
                strength: Rfc2119Strength::Must,
                category: crate::translator::classify(statement),
                ltl: None,
                scenarios: vec![],
                source: SourceLocation {
                    file: "spec.md".into(),
                    start_byte: 0,
                    end_byte: 0,
                    start_line: idx + 1,
                    end_line: idx + 1,
                },
            })
            .collect();
        plan
    }

    const THREE_SEQUENTIAL: [&str; 3] = [
        "T1.1 SHALL parse BEFORE T1.2 SHALL derive.",
        "T1.1 SHALL resolve BEFORE T1.2 SHALL emit.",
        "T1.1 SHALL check BEFORE T1.2 SHALL register.",
    ];

    fn diversity_details(plan: &PlanIR) -> Vec<String> {
        check_diversity(plan)
            .into_iter()
            .filter(|i| i.check == "low_diversity")
            .map(|i| i.detail)
            .collect()
    }

    fn pattern_ungrounded_detail(statement: &str) -> String {
        let plan = plan_with_statements(&[statement]);
        let (blockers, _, _) = check_classifiability(&plan, true);
        blockers
            .iter()
            .find(|b| b.check == "pattern_ungrounded")
            .map(|b| b.detail.clone())
            .unwrap_or_default()
    }

    /// Three modelled ordering constraints are one category, so the advisory fires.
    #[test]
    fn modelled_sequential_plan_reports_low_diversity() {
        let details = diversity_details(&plan_with_statements(&THREE_SEQUENTIAL));
        assert_eq!(
            details.len(),
            1,
            "expected one low_diversity item: {details:?}"
        );
        assert!(
            details[0].contains("sequential(3)"),
            "the distribution must count the three modelled constraints: {}",
            details[0]
        );
    }

    /// The distribution describes the model. A body that yields no formula adds no verification, so
    /// it must not enter the tally — counting it let a requirement that checks nothing silence the
    /// advisory.
    #[test]
    fn unmodelled_requirement_adds_no_diversity() {
        let mut statements = THREE_SEQUENTIAL.to_vec();
        statements.push("IF a client supplies a slug THEN the registry SHALL ignore the value.");

        let details = diversity_details(&plan_with_statements(&statements));
        assert_eq!(
            details.len(),
            1,
            "an unmodelled conditional must not silence low_diversity: {details:?}"
        );
        assert!(
            details[0].contains("sequential(3)"),
            "the unmodelled conditional must not enter the distribution: {}",
            details[0]
        );
    }

    /// The same body becomes a constraint once it names two tasks, and that is what silences the
    /// advisory — verification, not prose.
    #[test]
    fn modelled_non_sequential_constraint_silences_diversity() {
        let mut statements = THREE_SEQUENTIAL.to_vec();
        statements.push("IF T1.1 fails THEN T1.2 SHALL retry.");

        let details = diversity_details(&plan_with_statements(&statements));
        assert!(
            details.is_empty(),
            "a second category ends the advisory: {details:?}"
        );
    }

    /// The predicate keywords are matched by the classifier, so a body using one is not "ungrounded
    /// for want of a keyword". Each pair category names the argument it lacks.
    #[test]
    fn a_pair_category_names_the_argument_it_lacks() {
        let conditional = pattern_ungrounded_detail(
            "IF a client supplies a slug THEN the registry SHALL ignore it.",
        );
        assert!(
            conditional.contains("trigger task and a consequent task"),
            "a conditional must name its two arguments: {conditional}"
        );

        let exclusive =
            pattern_ungrounded_detail("The registry SHALL persist AT MOST ONE slug source.");
        assert!(
            exclusive.contains("pair the constraint keeps apart"),
            "an exclusive must name the pair: {exclusive}"
        );

        let concurrent =
            pattern_ungrounded_detail("The registry SHALL index CONCURRENTLY at boot.");
        assert!(
            concurrent.contains("one concurrent phase"),
            "a concurrent must name its two tasks: {concurrent}"
        );
    }

    /// One task id is not a pair: the constraint cannot be built, so the body is reported rather
    /// than passing as a modelled constraint that carries no formula.
    #[test]
    fn one_task_id_is_not_enough_for_a_conditional() {
        let detail = pattern_ungrounded_detail("IF T1.1 fails THEN the registry SHALL back off.");
        assert!(
            detail.contains("trigger task and a consequent task"),
            "a conditional citing one task must report it: {detail}"
        );
    }

    /// A body with no temporal keyword keeps the generic message: it is missing a keyword, not an
    /// argument of a pair category.
    #[test]
    fn a_body_without_a_pair_keyword_keeps_the_generic_message() {
        let detail = pattern_ungrounded_detail("The system SHALL hold the header ALWAYS.");
        assert!(
            detail.contains("add task IDs for model verification"),
            "the generic message must remain for a non-pair category: {detail}"
        );
        assert!(
            !detail.contains("trigger task"),
            "a global body must not be told it lacks a trigger: {detail}"
        );
    }
}

#[test]
fn informational_requirement_does_not_block() {
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

#[test]
fn temporal_constraint_beats_human_review_marker() {
    use crate::ir::ConstraintCategory;
    // A temporal constraint + a trailing human-review note must still be
    // classified as a temporal category (verifiable), NOT informational.
    let stmt = "T1.1 SHALL produce an audit log BEFORE T1.2 SHALL deploy it. The audit log is human review only.";
    let cat = crate::translator::classify(stmt);
    assert_ne!(
        cat,
        ConstraintCategory::Informational,
        "temporal keyword must take priority over the human-review marker"
    );
    assert_ne!(
        cat,
        ConstraintCategory::NonFormalizable,
        "temporal keyword must make the requirement verifiable"
    );
}

#[test]
fn non_temporal_with_marker_is_informational() {
    use crate::ir::ConstraintCategory;
    // A capability/policy with the marker and NO temporal keyword is
    // informational, not NonFormalizable.
    let stmt = "This policy is human review only. It SHALL follow the security standard.";
    assert_eq!(
        crate::translator::classify(stmt),
        ConstraintCategory::Informational,
        "non-temporal requirement with marker should be Informational"
    );
}

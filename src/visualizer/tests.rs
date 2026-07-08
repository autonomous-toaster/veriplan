use crate::visualizer::*;
use crate::ir::*;
use crate::ir::ltl::{LtlCondition, LtlFormula};

    #[test]
    fn test_clean_label() {
        assert_eq!(clean_label("hello"), "hello");
        assert_eq!(clean_label("`code`"), "'code'");
        assert_eq!(clean_label("a \"quote\""), "a 'quote'");
    }

    #[test]
    fn test_node_id() {
        let id = node_id("1.1");
        assert_eq!(id, "T1_1");
    }

    #[test]
    fn test_node_id_dot() {
        let id = node_id_dot("1.1");
        assert_eq!(id, "t_1_1");
    }

    #[test]
    fn test_escape_dot() {
        // escape_dot replaces quotes and newlines, not dots
        assert_eq!(escape_dot("1.1"), "1.1");
        assert_eq!(escape_dot("hello"), "hello");
        assert_eq!(escape_dot("a\"b"), "a\\\"b");
    }

    #[test]
    fn test_source_markdown_link() {
        let task = Task {
            id: "1.1".into(),
            description: "test".into(),
            phase: "Phase 1".into(),
            checked: false,
            source: SourceLocation {
                file: "spec.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 5,
                end_line: 5,
            },
        };
        let link = source_markdown_link(&task);
        assert_eq!(link, "spec.md#L5");
    }

    #[test]
    fn test_category_label() {
        assert_eq!(
            category_label(&ConstraintCategory::SequentialOrder),
            "sequential"
        );
        assert_eq!(
            category_label(&ConstraintCategory::NonFormalizable),
            "non-formalizable"
        );
        assert_eq!(category_label(&ConstraintCategory::Exclusive), "exclusive");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        let t = truncate("a very long string that should be truncated", 20);
        assert!(t.len() <= 23, "truncated string too long: {}", t);
    }

    #[test]
    fn test_write_mermaid_task_node_checked() {
        let mut s = String::new();
        let task = Task {
            id: "1.1".into(),
            description: "Setup".into(),
            phase: "Phase 1".into(),
            checked: true,
            source: SourceLocation {
                file: "tasks.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                end_line: 1,
            },
        };
        write_mermaid_task_node(&mut s, &task);
        assert!(s.contains("T1_1"));
        assert!(s.contains("\u{2705}"));
    }

    #[test]
    fn test_write_mermaid_task_node_unchecked() {
        let mut s = String::new();
        let task = Task {
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
        };
        write_mermaid_task_node(&mut s, &task);
        assert!(s.contains("T1_2"));
        assert!(!s.contains("\u{2705}"));
    }

    #[test]
    fn test_write_dot_task_node_checked() {
        let mut s = String::new();
        let task = Task {
            id: "1.1".into(),
            description: "Setup".into(),
            phase: "Phase 1".into(),
            checked: true,
            source: SourceLocation {
                file: "tasks.md".into(),
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                end_line: 1,
            },
        };
        write_dot_task_node(&mut s, &task);
        assert!(s.contains("t_1_1"));
        assert!(s.contains("#e1f5e1"));
    }

    #[test]
    fn test_write_dot_task_node_unchecked() {
        let mut s = String::new();
        let task = Task {
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
        };
        write_dot_task_node(&mut s, &task);
        assert!(s.contains("t_1_2"));
        assert!(!s.contains("#e1f5e1"));
    }

    #[test]
    fn test_display_label_sequential() {
        let c = TranslatedConstraint {
            requirement_id: "R1".into(),
            category: ConstraintCategory::SequentialOrder,
            statement: "T1.1 SHALL complete BEFORE T1.2".into(),
            ltl: Some(LtlFormula::Always(LtlCondition::Implies(
                Box::new(LtlCondition::Atom("active_t1_2".into())),
                Box::new(LtlCondition::Atom("done_t1_1".into())),
            ))),
            strength: Rfc2119Strength::Must,
            is_hard: true,
        };
        assert_eq!(display_label(&c), "sequential");
    }

    #[test]
    fn test_display_label_fixed_time_with_ordering() {
        let c = TranslatedConstraint {
            requirement_id: "R1".into(),
            category: ConstraintCategory::FixedTime,
            statement: "T1.1 SHALL complete BEFORE T1.2".into(),
            ltl: Some(LtlFormula::Always(LtlCondition::Implies(
                Box::new(LtlCondition::Atom("active_t1_2".into())),
                Box::new(LtlCondition::Atom("done_t1_1".into())),
            ))),
            strength: Rfc2119Strength::Must,
            is_hard: true,
        };
        assert_eq!(display_label(&c), "sequential");
    }

    #[test]
    fn test_display_label_fixed_time_no_ordering() {
        let c = TranslatedConstraint {
            requirement_id: "R1".into(),
            category: ConstraintCategory::FixedTime,
            statement: "The system SHALL respond within 5s".into(),
            ltl: Some(LtlFormula::Always(LtlCondition::Eventually(Box::new(LtlCondition::Atom("done_t1_1".into()))))),
            strength: Rfc2119Strength::Must,
            is_hard: true,
        };
        assert_eq!(display_label(&c), "fixed-time");
    }

    fn make_simple_plan() -> PlanIR {
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
                    checked: true,
                    source: SourceLocation {
                        file: "tasks.md".into(),
                        start_byte: 0,
                        end_byte: 0,
                        start_line: 2,
                        end_line: 2,
                    },
                },
            ],
            requirements: vec![],
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
    fn test_format_mermaid_contains_phase() {
        let plan = make_simple_plan();
        let constraints = vec![];
        let result = format_mermaid(&plan, &constraints);
        assert!(result.contains("Phase 1"));
        assert!(result.contains("T1_1"));
        assert!(result.contains("T1_2"));
    }

    #[test]
    fn test_format_dot_contains_phase() {
        let plan = make_simple_plan();
        let constraints = vec![];
        let result = format_dot(&plan, &constraints);
        assert!(result.contains("Phase 1"));
        assert!(result.contains("t_1_1"));
        assert!(result.contains("t_1_2"));
    }

    #[test]
    fn test_format_markdown_contains_phase() {
        let plan = make_simple_plan();
        let constraints = vec![];
        let result = format_markdown(&plan, &constraints);
        assert!(result.contains("Phase 1"));
        assert!(result.contains("T1.1"));
        assert!(result.contains("T1.2"));
    }

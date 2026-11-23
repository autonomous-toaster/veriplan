#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // Tests fail by panicking; the crate denies these for code that ships.

use crate::checker::bfs::*;
#[test]
fn test_extract_task_ids_t_prefix() {
    let ids = extract_task_ids("T1.1 SHALL complete BEFORE T1.2");
    assert_eq!(ids, vec!["1.1", "1.2"]);
}
#[test]
fn test_extract_task_ids_ltl_format() {
    let ids = extract_task_ids("G (active_t1_1 -> done_t1_2)");
    assert_eq!(ids, vec!["1.1", "1.2"]);
}
#[cfg(test)]
mod suggest_tests {
    use crate::checker::bfs::*;
    use crate::ir::ltl::{LtlCondition, LtlFormula};
    #[test]
    fn test_suggest_fix_sequential() {
        let fix = suggest_fix(
            &ConstraintCategory::SequentialOrder,
            "[] ( active_t1_2 -> done_t1_1 )",
            "R1",
            "T1.1 SHALL complete BEFORE T1.2",
        );
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("before-task"));
    }
    #[test]
    fn test_suggest_fix_exclusive() {
        let fix = suggest_fix(
            &ConstraintCategory::Exclusive,
            "[] ( !(active_t2_1 && active_t2_2) )",
            "R1",
            "At most one of T2.1, T2.2 SHALL be active",
        );
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("mutually exclusive"));
    }
    #[test]
    fn test_suggest_fix_exclusive_only_one() {
        let fix = suggest_fix(
            &ConstraintCategory::Exclusive,
            "[] ( !(active_t2_1 && active_t2_2) )",
            "R1",
            "IF the messages array contains only one message",
        );
        assert!(fix.is_some());
        let msg = fix.unwrap();
        assert!(
            msg.contains("only one"),
            "Expected 'only one' in message: {}",
            msg
        );
        // Should not mention AT MOST ONE as the detected trigger
        assert!(
            msg.find("body text contains 'only one'").unwrap_or(0)
                < msg.find("AT MOST ONE").unwrap_or(usize::MAX),
            "body text detection should come before generic AT MOST ONE reference"
        );
    }
    #[test]
    fn test_suggest_fix_concurrent() {
        let fix = suggest_fix(
            &ConstraintCategory::ConcurrentEvents,
            "[] ( active_t3_1 <-> active_t3_2 )",
            "R1",
            "T3.1 and T3.2 SHALL run concurrently",
        );
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("CONCURRENTLY"));
    }
    #[test]
    fn test_suggest_fix_conditional() {
        let fix = suggest_fix(
            &ConstraintCategory::Conditional,
            "[] ( failed_t1_1 -> <> active_t2_1 )",
            "R1",
            "IF T1.1 fails THEN T2.1 SHALL run",
        );
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("IF"));
    }
    #[test]
    fn test_suggest_fix_conditional_if_in_body() {
        let fix = suggest_fix(
            &ConstraintCategory::Conditional,
            "[] ( failed_t1_1 -> <> active_t2_1 )",
            "R1",
            "T1.1 SHALL complete BEFORE T1.2. If absent, the handler MUST return 400.",
        );
        assert!(fix.is_some());
        let msg = fix.unwrap();
        assert!(
            msg.contains("body text contains 'if'"),
            "Expected body text message: {}",
            msg
        );
        // The message should identify the trigger before explaining IF...THEN
        assert!(
            msg.find("body text contains 'if'").unwrap_or(0)
                < msg.find("IF...THEN is designed").unwrap_or(usize::MAX),
            "body text detection should come before IF...THEN explanation"
        );
    }
    #[test]
    fn test_suggest_fix_global() {
        let fix = suggest_fix(
            &ConstraintCategory::Global,
            "true",
            "R1",
            "T1.1 SHALL ALWAYS be available",
        );
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("global invariant"));
    }
    #[test]
    fn test_suggest_fix_fixed_time() {
        let fix = suggest_fix(
            &ConstraintCategory::FixedTime,
            "true",
            "R1",
            "T1.1 SHALL complete within the 2-4 AM window",
        );
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("fixed-time"));
    }
    fn make_state() -> Vec<(String, u8)> {
        vec![
            ("active_t1_1".into(), 1),
            ("done_t1_1".into(), 0),
            ("active_t1_2".into(), 0),
            ("done_t1_2".into(), 1),
        ]
    }
    #[test]
    fn test_evaluate_ltl_atom_true() {
        let state = make_state();
        assert!(evaluate_ltl_atom("active_t1_1", &state));
    }
    #[test]
    fn test_evaluate_ltl_atom_false() {
        let state = make_state();
        assert!(!evaluate_ltl_atom("active_t1_2", &state));
    }
    #[test]
    fn test_evaluate_ltl_atom_negation() {
        let state = make_state();
        assert!(evaluate_ltl_atom("!active_t1_2", &state));
        assert!(!evaluate_ltl_atom("!active_t1_1", &state));
    }
    #[test]
    fn test_evaluate_ltl_atom_unknown() {
        let state = make_state();
        assert!(!evaluate_ltl_atom("nonexistent", &state));
    }
    #[test]
    fn test_evaluate_ltl_condition_implication() {
        let state = make_state();
        // active_t1_1 -> done_t1_2: true -> true = true
        let cond = LtlCondition::Implies(
            Box::new(LtlCondition::Atom("active_t1_1".into())),
            Box::new(LtlCondition::Atom("done_t1_2".into())),
        );
        assert!(evaluate_ltl_condition(&cond, &state));
        // active_t1_2 -> done_t1_1: false -> false = true
        let cond = LtlCondition::Implies(
            Box::new(LtlCondition::Atom("active_t1_2".into())),
            Box::new(LtlCondition::Atom("done_t1_1".into())),
        );
        assert!(evaluate_ltl_condition(&cond, &state));
        // active_t1_1 -> done_t1_1: true -> false = false
        let cond = LtlCondition::Implies(
            Box::new(LtlCondition::Atom("active_t1_1".into())),
            Box::new(LtlCondition::Atom("done_t1_1".into())),
        );
        assert!(!evaluate_ltl_condition(&cond, &state));
    }
    #[test]
    fn test_evaluate_ltl_condition_bidirectional() {
        let state = make_state();
        // active_t1_1 <-> done_t1_2: 1 == 1 = true
        let cond = LtlCondition::Iff(
            Box::new(LtlCondition::Atom("active_t1_1".into())),
            Box::new(LtlCondition::Atom("done_t1_2".into())),
        );
        assert!(evaluate_ltl_condition(&cond, &state));
        // active_t1_1 <-> active_t1_2: 1 == 0 = false
        let cond = LtlCondition::Iff(
            Box::new(LtlCondition::Atom("active_t1_1".into())),
            Box::new(LtlCondition::Atom("active_t1_2".into())),
        );
        assert!(!evaluate_ltl_condition(&cond, &state));
    }
    #[test]
    fn test_evaluate_ltl_condition_negation() {
        let state = make_state();
        let cond = LtlCondition::Not(Box::new(LtlCondition::Atom("active_t1_2".into())));
        assert!(evaluate_ltl_condition(&cond, &state));
        let cond = LtlCondition::Not(Box::new(LtlCondition::Atom("active_t1_1".into())));
        assert!(!evaluate_ltl_condition(&cond, &state));
    }
    #[test]
    fn test_evaluate_ltl_condition_and() {
        let state = make_state();
        let cond = LtlCondition::And(vec![
            LtlCondition::Atom("active_t1_1".into()),
            LtlCondition::Atom("done_t1_2".into()),
        ]);
        assert!(evaluate_ltl_condition(&cond, &state));
        let cond = LtlCondition::And(vec![
            LtlCondition::Atom("active_t1_1".into()),
            LtlCondition::Atom("active_t1_2".into()),
        ]);
        assert!(!evaluate_ltl_condition(&cond, &state));
    }
    #[test]
    fn test_evaluate_ltl_condition_eventually() {
        let state = make_state();
        let cond = LtlCondition::Eventually(Box::new(LtlCondition::Atom("active_t1_1".into())));
        assert!(evaluate_ltl_condition(&cond, &state));
        let cond = LtlCondition::Eventually(Box::new(LtlCondition::Atom("active_t1_2".into())));
        assert!(!evaluate_ltl_condition(&cond, &state));
    }
    #[test]
    fn test_evaluate_ltl_always() {
        let state = make_state();
        // [] ( active_t1_1 -> done_t1_2 ): true -> true = true
        let formula = LtlFormula::Always(LtlCondition::Implies(
            Box::new(LtlCondition::Atom("active_t1_1".into())),
            Box::new(LtlCondition::Atom("done_t1_2".into())),
        ));
        assert!(evaluate_ltl(
            &formula,
            &state,
            &PlanIR {
                tasks: vec![],
                requirements: vec![],
                scenarios: vec![],
                phases: vec![],
                source_map: SourceMap::default()
            }
        ));
        // [] ( active_t1_1 -> done_t1_1 ): true -> false = false
        let formula = LtlFormula::Always(LtlCondition::Implies(
            Box::new(LtlCondition::Atom("active_t1_1".into())),
            Box::new(LtlCondition::Atom("done_t1_1".into())),
        ));
        assert!(!evaluate_ltl(
            &formula,
            &state,
            &PlanIR {
                tasks: vec![],
                requirements: vec![],
                scenarios: vec![],
                phases: vec![],
                source_map: SourceMap::default()
            }
        ));
    }
    #[test]
    fn test_evaluate_ltl_eventually() {
        let state = make_state();
        // [] ( <> active_t1_1 ): true = true
        let formula = LtlFormula::Always(LtlCondition::Eventually(Box::new(LtlCondition::Atom(
            "active_t1_1".into(),
        ))));
        assert!(evaluate_ltl(
            &formula,
            &state,
            &PlanIR {
                tasks: vec![],
                requirements: vec![],
                scenarios: vec![],
                phases: vec![],
                source_map: SourceMap::default()
            }
        ));
        // [] ( <> active_t1_2 ): false = false
        let formula = LtlFormula::Always(LtlCondition::Eventually(Box::new(LtlCondition::Atom(
            "active_t1_2".into(),
        ))));
        assert!(!evaluate_ltl(
            &formula,
            &state,
            &PlanIR {
                tasks: vec![],
                requirements: vec![],
                scenarios: vec![],
                phases: vec![],
                source_map: SourceMap::default()
            }
        ));
    }
}

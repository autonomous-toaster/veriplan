#[cfg(test)]
mod tests {
    use crate::checker::bfs::*;
    use crate::ir::*;

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

    #[test]
    fn test_extract_task_ids_empty() {
        let ids = extract_task_ids("No task IDs here");
        assert!(ids.is_empty());
    }

    #[test]
    fn test_extract_task_ids_mixed() {
        let ids = extract_task_ids("T10.7 and T3.2");
        assert_eq!(ids, vec!["10.7", "3.2"]);
    }
}

#[cfg(test)]
mod suggest_tests {
    use crate::checker::bfs::*;
    use crate::ir::*;

    #[test]
    fn test_suggest_fix_sequential() {
        let fix = suggest_fix(&ConstraintCategory::SequentialOrder, "[] ( active_t1_2 -> done_t1_1 )", "R1");
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("before-task"));
    }

    #[test]
    fn test_suggest_fix_exclusive() {
        let fix = suggest_fix(&ConstraintCategory::Exclusive, "[] ( !(active_t2_1 && active_t2_2) )", "R1");
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("mutually exclusive"));
    }

    #[test]
    fn test_suggest_fix_concurrent() {
        let fix = suggest_fix(&ConstraintCategory::ConcurrentEvents, "[] ( active_t3_1 <-> active_t3_2 )", "R1");
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("CONCURRENTLY"));
    }

    #[test]
    fn test_suggest_fix_conditional() {
        let fix = suggest_fix(&ConstraintCategory::Conditional, "[] ( failed_t1_1 -> <> active_t2_1 )", "R1");
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("IF"));
    }

    #[test]
    fn test_suggest_fix_global() {
        let fix = suggest_fix(&ConstraintCategory::Global, "true", "R1");
        assert!(fix.is_none());
    }

    fn make_state() -> HashMap<String, u8> {
        let mut state = HashMap::new();
        state.insert("active_t1_1".into(), 1);
        state.insert("done_t1_1".into(), 0);
        state.insert("active_t1_2".into(), 0);
        state.insert("done_t1_2".into(), 1);
        state
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
        assert!(evaluate_ltl_condition("active_t1_1 -> done_t1_2", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
        // active_t1_2 -> done_t1_1: false -> false = true
        assert!(evaluate_ltl_condition("active_t1_2 -> done_t1_1", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
        // active_t1_1 -> done_t1_1: true -> false = false
        assert!(!evaluate_ltl_condition("active_t1_1 -> done_t1_1", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
    }

    #[test]
    fn test_evaluate_ltl_condition_bidirectional() {
        let state = make_state();
        // active_t1_1 <-> done_t1_2: 1 == 1 = true
        assert!(evaluate_ltl_condition("active_t1_1 <-> done_t1_2", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
        // active_t1_1 <-> active_t1_2: 1 == 0 = false
        assert!(!evaluate_ltl_condition("active_t1_1 <-> active_t1_2", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
    }

    #[test]
    fn test_evaluate_ltl_condition_negation() {
        let state = make_state();
        assert!(evaluate_ltl_condition("!(active_t1_2)", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
        assert!(!evaluate_ltl_condition("!(active_t1_1)", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
    }

    #[test]
    fn test_evaluate_ltl_condition_and() {
        let state = make_state();
        assert!(evaluate_ltl_condition("active_t1_1 && done_t1_2", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
        assert!(!evaluate_ltl_condition("active_t1_1 && active_t1_2", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
    }

    #[test]
    fn test_evaluate_ltl_condition_eventually() {
        let state = make_state();
        assert!(evaluate_ltl_condition("F active_t1_1", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
        assert!(!evaluate_ltl_condition("F active_t1_2", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
    }

    #[test]
    fn test_evaluate_ltl_always() {
        let state = make_state();
        // G ( active_t1_1 -> done_t1_2 )
        let ltl = "G ( active_t1_1 -> done_t1_2 )";
        assert!(evaluate_ltl(ltl, &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
        // G ( active_t1_1 -> done_t1_1 )
        let ltl2 = "G ( active_t1_1 -> done_t1_1 )";
        assert!(!evaluate_ltl(ltl2, &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
    }

    #[test]
    fn test_evaluate_ltl_unrecognized() {
        let state = make_state();
        // Unrecognized patterns pass conservatively
        assert!(evaluate_ltl("unknown pattern", &state, &PlanIR { tasks: vec![], requirements: vec![], scenarios: vec![], phases: vec![], source_map: SourceMap::default() }));
    }

    fn make_plan_with_phases() -> PlanIR {
        PlanIR {
            tasks: vec![
                Task { id: "1.1".into(), description: "Setup".into(), phase: "Phase 1".into(), checked: false, source: SourceLocation { file: "t.md".into(), start_byte: 0, end_byte: 0, start_line: 1, end_line: 1 } },
                Task { id: "1.2".into(), description: "Build".into(), phase: "Phase 1".into(), checked: false, source: SourceLocation { file: "t.md".into(), start_byte: 0, end_byte: 0, start_line: 2, end_line: 2 } },
                Task { id: "2.1".into(), description: "Deploy".into(), phase: "Phase 2".into(), checked: false, source: SourceLocation { file: "t.md".into(), start_byte: 0, end_byte: 0, start_line: 3, end_line: 3 } },
            ],
            requirements: vec![],
            scenarios: vec![],
            phases: vec![
                Phase { name: "Phase 1".into(), task_ids: vec!["1.1".into(), "1.2".into()], mode: PhaseMode::Sequential },
                Phase { name: "Phase 2".into(), task_ids: vec!["2.1".into()], mode: PhaseMode::Sequential },
            ],
            source_map: SourceMap::default(),
        }
    }

    #[test]
    fn test_find_predecessors_first_in_phase() {
        let plan = make_plan_with_phases();
        let preds = find_predecessors(&plan, "1.1");
        // First task in first phase: no predecessor
        assert!(preds.is_empty());
    }

    #[test]
    fn test_find_predecessors_second_in_phase() {
        let plan = make_plan_with_phases();
        let preds = find_predecessors(&plan, "1.2");
        assert_eq!(preds, vec!["1.1"]);
    }

    #[test]
    fn test_find_predecessors_first_in_second_phase() {
        let plan = make_plan_with_phases();
        let preds = find_predecessors(&plan, "2.1");
        assert_eq!(preds, vec!["1.2"]);
    }

    #[test]
    fn test_find_predecessors_unknown_task() {
        let plan = make_plan_with_phases();
        let preds = find_predecessors(&plan, "99.9");
        assert!(preds.is_empty());
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let t = truncate("hello world", 5);
        assert_eq!(t, "hello...");
    }
}

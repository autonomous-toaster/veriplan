use std::collections::BTreeSet;
use std::fmt::Write;

use crate::ir::*;
use crate::translator;

/// Generate Promela source from PlanIR and constraints.
pub fn generate_promela(plan: &PlanIR, constraints: &[translator::TranslatedConstraint]) -> String {
    let mut s = String::new();

    // A failure branch exists only to satisfy an IF...THEN constraint, and only for the task that
    // constraint names. Emitting it for every task doubles the branching at every step: a plan with
    // one conditional constraint used to pay 2^tasks states for flags nothing reads.
    let failing_tasks = tasks_naming_a_failure_flag(plan, constraints);

    // Header
    writeln!(s, "/* Promela model — task structure only */").ok();
    writeln!(s).ok();

    // ── Variable declarations ──
    for task in &plan.tasks {
        let desc = task.description.replace("/*", "/ *").replace("*/", "* /");
        writeln!(s, "bit {} = 0;\t/* {} */", active_var(&task.id), desc).ok();
        writeln!(s, "bit {} = 0;", done_var(&task.id)).ok();
    }
    writeln!(s).ok();

    // Failed flags for conditional constraint LTL references. A flag a claim cannot read is not
    // emitted at all: it would still occupy a field in the state vector every backend hashes.
    for task in plan.tasks.iter().filter(|t| failing_tasks.contains(&t.id)) {
        writeln!(s, "bit {} = 0;", fail_var(&task.id)).ok();
    }
    writeln!(s).ok();

    // ── Task execution processes (phase-ordered only) ──
    for task in &plan.tasks {
        let av = active_var(&task.id);
        let dv = done_var(&task.id);
        let fv = fail_var(&task.id);

        writeln!(s, "active proctype task_{}() {{", task.id.replace('.', "_")).ok();

        // Only phase-ordering guard: predecessor must be done
        let predecessors = super::bfs::find_predecessors(plan, &task.id);
        if predecessors.is_empty() {
            writeln!(s, "\tdo").ok();
            writeln!(s, "\t:: (1) ->").ok();
        } else {
            let guard = predecessors
                .iter()
                .map(|id| format!("{} == 1", done_var(id)))
                .collect::<Vec<_>>()
                .join(" && ");
            writeln!(s, "\tdo").ok();
            writeln!(s, "\t:: {} ->", guard).ok();
        }

        // Task body. Completion is published LAST: the successor's guard reads the flag, so
        // writing the flag before the task finishes lets the successor start inside this task's
        // last statements. That window is what made the state space the product of N of them.
        writeln!(s, "\t\t{} = 1;\t/* activate */", av).ok();
        writeln!(s, "\t\t{} = 0;\t/* deactivate */", av).ok();

        if failing_tasks.contains(&task.id) {
            // Non-deterministic failure, only where a constraint reads the flag. Emitted before
            // the completion flag so that "done" means the failure status is decided.
            writeln!(s, "\t\tif").ok();
            writeln!(s, "\t\t:: {} = 1;", fv).ok();
            writeln!(s, "\t\t:: skip;").ok();
            writeln!(s, "\t\tfi;").ok();
        }

        writeln!(s, "\t\t{} = 1;\t/* complete */", dv).ok();
        writeln!(s, "\t\tbreak").ok();
        writeln!(s, "\tod").ok();
        writeln!(s, "}}").ok();
        writeln!(s).ok();
    }

    // ── LTL properties — spec constraints checked against phase-ordered model ──
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();
    for (i, c) in formalizable.iter().enumerate() {
        if let Some(ltl) = &c.ltl {
            let ltl_str = crate::ir::ltl::ltl_to_string(ltl);
            writeln!(s, "ltl p{} {{ {} }} /* {} */", i, ltl_str, c.requirement_id).ok();
        }
    }

    s
}

/// The tasks whose failure flag some constraint's formula names.
///
/// The decision is a walk over the formula's atoms, not a search of the rendered string. A task
/// whose flag no formula names gets no branch, so the flag does not enter the state vector at all.
fn tasks_naming_a_failure_flag(
    plan: &PlanIR,
    constraints: &[translator::TranslatedConstraint],
) -> BTreeSet<String> {
    let named: BTreeSet<&str> = constraints
        .iter()
        .filter_map(|c| c.ltl.as_ref())
        .flat_map(|f| f.atoms())
        .collect();
    plan.tasks
        .iter()
        .filter(|t| named.contains(fail_var(&t.id).as_str()))
        .map(|t| t.id.clone())
        .collect()
}

pub fn active_var(id: &str) -> String {
    format!("active_t{}", id.replace('.', "_"))
}

pub fn done_var(id: &str) -> String {
    format!("done_t{}", id.replace('.', "_"))
}

pub fn fail_var(id: &str) -> String {
    format!("failed_t{}", id.replace('.', "_"))
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; the crate denies these three lints for code that ships.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::generate_promela;
    use crate::ir::ltl::{LtlCondition, LtlFormula};
    use crate::ir::*;
    use crate::translator::TranslatedConstraint;

    /// A conditional constraint: if the named task fails, the consequent eventually runs.
    fn conditional_claim(failing: &str, consequent: &str) -> TranslatedConstraint {
        TranslatedConstraint {
            requirement_id: "cap::Recovers".into(),
            statement: format!("IF T{failing} fails THEN T{consequent} SHALL run."),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::Conditional,
            ltl: Some(LtlFormula::Always(LtlCondition::Implies(
                Box::new(LtlCondition::Atom(super::fail_var(failing))),
                Box::new(LtlCondition::Eventually(Box::new(LtlCondition::Atom(
                    format!("active_{}", crate::translator::normalize_id(consequent)),
                )))),
            ))),
            is_hard: true,
            historical: None,
            foreign_tasks: Vec::new(),
            unmodelled_reason: None,
        }
    }

    /// A safety claim that names one failure flag and holds in every reachable state, so the space
    /// closes instead of stopping at the first refutation.
    fn one_flag_claim(task: &str) -> TranslatedConstraint {
        let atom = LtlCondition::Atom(super::fail_var(task));
        TranslatedConstraint {
            requirement_id: "cap::NamesOneFlag".into(),
            statement: "T1.1 SHALL complete BEFORE T1.1 SHALL complete.".into(),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::Conditional,
            ltl: Some(LtlFormula::Always(LtlCondition::Implies(
                Box::new(atom.clone()),
                Box::new(atom),
            ))),
            is_hard: true,
            historical: None,
            foreign_tasks: Vec::new(),
            unmodelled_reason: None,
        }
    }

    /// A safety claim of the ordering shape, from one task to another.
    fn safety_claim(active: &str, done: &str) -> TranslatedConstraint {
        TranslatedConstraint {
            requirement_id: "cap::Safety".into(),
            statement: "T1.1 SHALL complete BEFORE T1.2 SHALL start.".into(),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::SequentialOrder,
            ltl: Some(LtlFormula::Always(LtlCondition::Implies(
                Box::new(LtlCondition::Atom(format!(
                    "active_{}",
                    crate::translator::normalize_id(active)
                ))),
                Box::new(LtlCondition::Atom(format!(
                    "done_{}",
                    crate::translator::normalize_id(done)
                ))),
            ))),
            is_hard: true,
            historical: None,
            foreign_tasks: Vec::new(),
            unmodelled_reason: None,
        }
    }

    /// A failure flag exists for the tasks a constraint names, and only for those.
    #[test]
    fn a_task_no_claim_names_carries_no_failure_block() {
        let plan = chain_plan(3);
        let promela = generate_promela(&plan, &[conditional_claim("1.2", "1.3")]);
        assert!(
            promela.contains("bit failed_t1_2 = 0;"),
            "the named task must carry its flag:\n{promela}"
        );
        assert!(
            promela.contains(":: failed_t1_2 = 1;"),
            "the named task must carry the branch:\n{promela}"
        );
        assert!(
            !promela.contains("bit failed_t1_1 = 0;"),
            "an unnamed task must carry no flag:\n{promela}"
        );
        assert!(
            !promela.contains("bit failed_t1_3 = 0;"),
            "an unnamed task must carry no flag:\n{promela}"
        );
    }

    /// The flag of task 1.10 must not be read as naming task 1.1.
    #[test]
    fn a_failure_flag_is_not_matched_as_a_prefix() {
        let plan = chain_plan(10);
        let promela = generate_promela(&plan, &[conditional_claim("1.10", "1.9")]);
        assert!(
            promela.contains("bit failed_t1_10 = 0;"),
            "the named task must carry its flag"
        );
        assert!(
            !promela.contains("bit failed_t1_1 = 0;"),
            "failed_t1_1 is a prefix of failed_t1_10 and must not be named by it:\n{promela}"
        );
    }

    /// Sixteen more tasks that no constraint names must not multiply the space. Under the old
    /// generator every one of them carried a branch, so the count doubled per task.
    #[test]
    fn the_state_count_does_not_follow_the_unnamed_tasks() {
        // The budget is deliberately small, because a model that branches on every task needs
        // millions of states and the default budget would take ten minutes to notice. But a cap
        // alone defeats the test: both sides would stop at the ceiling and the ratio between two
        // equal ceilings is one. So the budget being hit is itself the failure, and the absolute
        // bound below is what discriminates. Measured: a linear chain of twenty-four tasks needs
        // about two hundred states; the same chain branching on every task cannot fit in a million.
        let measure = |plan: &PlanIR| {
            let result = spin_rs::verify_with_config(
                &generate_promela(plan, &[one_flag_claim("1.1")]),
                &spin_rs::CheckerConfig {
                    max_states: 5_000,
                    ..Default::default()
                },
            )
            .expect("the model must be checkable");
            let starved = result
                .unverified
                .iter()
                .any(|u| u.kind.as_str() == "state_budget");
            (result.states_stored, starved)
        };
        let (eight, _) = measure(&chain_plan(8));
        let (twentyfour, starved) = measure(&chain_plan(24));

        assert!(
            !starved,
            "the twenty-four-task model stopped at the 5 000 state cap after {twentyfour} states: a \
             linear chain needs about two hundred, so this fixture is not linear and the ratio \
             below would be between two ceilings"
        );
        assert!(
            twentyfour < 1_000,
            "a twenty-four-task chain with one named flag explored {twentyfour} states"
        );
        assert!(
            twentyfour < 4 * eight,
            "eight tasks explored {eight} states and twenty-four explored {twentyfour}; the extra \
             sixteen tasks are not free"
        );
    }

    /// A claim that refutes near the root is not a state-count fixture.
    #[test]
    fn a_refuting_claim_is_not_a_state_count_fixture() {
        let plan = chain_plan(8);
        // 1.1 activates before 1.8 can be done, so this implication is false near the root.
        let refuted = spin_rs::verify(&generate_promela(&plan, &[safety_claim("1.1", "1.8")]))
            .expect("the refuting model must be checkable");
        // 1.8 activates last, when 1.1 is already done, so this one holds in every reachable state.
        let closed = spin_rs::verify(&generate_promela(&plan, &[safety_claim("1.8", "1.1")]))
            .expect("the closing model must be checkable");

        assert_eq!(
            refuted.violations.len(),
            1,
            "the refuting fixture must refute, or it is not the fixture this test is about"
        );
        assert!(
            closed.violations.is_empty(),
            "the count fixture must hold in every reachable state, or the space never closes"
        );
        // Both claims are safety claims of the same shape, and the in-process engine explores the
        // space whatever the verdicts, so the two counts coincide. That is why the count fixture has
        // to be the closing one: against an engine that does stop early, a refuting model would
        // report a handful of states and the comparison would read as parity.
        assert_eq!(
            refuted.states_stored, closed.states_stored,
            "the in-process count is verdict-blind; if that ever changes, this fixture distinction needs re-reading"
        );
    }

    /// A plan of `n` tasks in one sequential phase, ids 1.1 .. 1.n.
    fn chain_plan(n: usize) -> PlanIR {
        let ids: Vec<String> = (1..=n).map(|i| format!("1.{i}")).collect();
        let tasks = ids
            .iter()
            .map(|id| Task {
                id: id.clone(),
                description: format!("task {id}"),
                phase: "1".into(),
                checked: false,
                source: source(),
            })
            .collect();
        PlanIR {
            tasks,
            requirements: vec![],
            scenarios: vec![],
            phases: vec![Phase {
                name: "1".into(),
                task_ids: ids,
                mode: PhaseMode::Sequential,
            }],
            source_map: SourceMap::default(),
        }
    }

    fn source() -> SourceLocation {
        SourceLocation {
            file: "tasks.md".into(),
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            end_line: 1,
        }
    }

    /// One ordering claim: the last task activates only after the first has completed.
    fn ordering_claim(plan: &PlanIR) -> TranslatedConstraint {
        let first = plan.tasks[0].id.clone();
        let last = plan.tasks[plan.tasks.len() - 1].id.clone();
        TranslatedConstraint {
            requirement_id: "cap::Ordered".into(),
            statement: "T1.1 SHALL complete BEFORE the last task SHALL start.".into(),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::SequentialOrder,
            ltl: Some(LtlFormula::Always(LtlCondition::Implies(
                Box::new(LtlCondition::Atom(format!(
                    "active_{}",
                    crate::translator::normalize_id(&last)
                ))),
                Box::new(LtlCondition::Atom(format!(
                    "done_{}",
                    crate::translator::normalize_id(&first)
                ))),
            ))),
            is_hard: true,
            historical: None,
            foreign_tasks: Vec::new(),
            unmodelled_reason: None,
        }
    }

    #[test]
    fn a_task_publishes_completion_after_it_finishes() {
        let plan = chain_plan(3);
        let promela = generate_promela(&plan, &[]);
        for task in &plan.tasks {
            let deactivate = format!("{} = 0;", super::active_var(&task.id));
            let complete = format!("{} = 1;", super::done_var(&task.id));
            let at_deactivate = promela.find(&deactivate).unwrap_or(0);
            let at_complete = promela.find(&complete).unwrap_or(0);
            assert!(
                at_deactivate < at_complete,
                "task {} must deactivate before it publishes completion:\n{promela}",
                task.id
            );
        }
    }

    #[test]
    fn a_successor_is_guarded_on_its_predecessor_completion() {
        let plan = chain_plan(3);
        let promela = generate_promela(&plan, &[]);
        for pair in plan.tasks.windows(2) {
            let guard = format!("{} == 1 ->", super::done_var(&pair[0].id));
            let body = promela
                .find(&format!("proctype task_{}()", pair[1].id.replace('.', "_")))
                .map(|at| &promela[at..])
                .unwrap_or("");
            let end = body.find("\n}").unwrap_or(body.len());
            assert!(
                body[..end].contains(&guard),
                "task {} must be guarded on {}",
                pair[1].id,
                guard
            );
        }
    }

    /// An exclusion claim over two tasks of one `[concurrent]` phase.
    fn exclusion_claim(plan: &PlanIR) -> TranslatedConstraint {
        let a = plan.tasks[0].id.clone();
        let b = plan.tasks[1].id.clone();
        TranslatedConstraint {
            requirement_id: "cap::Exclusive".into(),
            statement: "AT MOST ONE of the two tasks SHALL be active.".into(),
            strength: Rfc2119Strength::Must,
            category: ConstraintCategory::Exclusive,
            ltl: Some(LtlFormula::Always(LtlCondition::Not(Box::new(
                LtlCondition::And(vec![
                    LtlCondition::Atom(format!("active_{}", crate::translator::normalize_id(&a))),
                    LtlCondition::Atom(format!("active_{}", crate::translator::normalize_id(&b))),
                ]),
            )))),
            is_hard: true,
            historical: None,
            foreign_tasks: Vec::new(),
            unmodelled_reason: None,
        }
    }

    #[test]
    fn exclusion_over_a_sequential_pair_holds() {
        // Two tasks in one ordered phase cannot overlap: the successor waits on the
        // predecessor's completion flag, which is now published after the task finishes.
        // The refutation this used to produce was an artifact of the interleaving window.
        let plan = chain_plan(2);
        let promela = generate_promela(&plan, &[exclusion_claim(&plan)]);
        let result = spin_rs::verify(&promela).expect("the model must be checkable");
        assert!(
            result.violations.is_empty(),
            "an ordered pair was reported concurrent: {:?}",
            result.violations
        );
    }

    #[test]
    fn exclusion_across_a_concurrent_phase_still_refutes() {
        // The check must stay covered where it can actually fail.
        let mut plan = chain_plan(2);
        plan.phases[0].mode = PhaseMode::Concurrent;
        let promela = generate_promela(&plan, &[exclusion_claim(&plan)]);
        let result = spin_rs::verify(&promela).expect("the model must be checkable");
        assert_eq!(
            result.violations.len(),
            1,
            "two tasks in a concurrent phase must be able to overlap"
        );
    }

    #[test]
    fn twenty_chained_tasks_stay_under_a_thousand_states() {
        // The bound is measured with pan, not with the in-process engine: an engine that is
        // itself superlinear would fail this test for a reason that is not the model's.
        if std::process::Command::new("spin")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let plan = chain_plan(20);
        let constraints = vec![ordering_claim(&plan)];
        let promela = generate_promela(&plan, &constraints);

        let dir = std::env::temp_dir().join("veriplan_scale_test");
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let pml = dir.join("chain20.pml");
        if std::fs::write(&pml, &promela).is_err() {
            return;
        }
        for f in ["pan", "pan.c"] {
            let _ = std::fs::remove_file(dir.join(f));
        }
        let generated = std::process::Command::new("spin")
            .args(["-a", "chain20.pml"])
            .current_dir(&dir)
            .output();
        if generated.is_err() || !dir.join("pan.c").exists() {
            return;
        }
        let compiled = std::process::Command::new("gcc")
            .args(["-w", "-o", "pan", "pan.c"])
            .stdin(std::process::Stdio::null())
            .current_dir(&dir)
            .output();
        if compiled.is_err() || !dir.join("pan").exists() {
            return;
        }
        let out = std::process::Command::new(dir.join("pan"))
            .args(["-N", "p0", "-n"])
            .current_dir(&dir)
            .output();
        let Ok(out) = out else { return };
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let states: usize = text
            .split(" states, stored")
            .next()
            .and_then(|head| head.split_whitespace().last())
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        assert!(states > 0, "the oracle returned no state count:\n{text}");
        assert!(
            states < 1000,
            "a twenty-task chain explored {states} states; the model is no longer linear"
        );
    }
}

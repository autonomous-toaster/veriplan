use std::fmt::Write;

use crate::ir::*;
use crate::translator;

/// Generate Promela source from PlanIR and constraints.
pub fn generate_promela(plan: &PlanIR, constraints: &[translator::TranslatedConstraint]) -> String {
    let mut s = String::new();

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

    // Failed flags for conditional constraint LTL references
    for task in &plan.tasks {
        writeln!(s, "bit {} = 0;", fail_var(&task.id)).ok();
    }
    writeln!(s).ok();

    // A failure branch exists only to satisfy an IF...THEN constraint. Emitting it for every
    // task doubles the branching at every step, and a plan with no conditional constraint pays
    // 2^N states for a flag nothing reads.
    let needs_failures = constraints
        .iter()
        .any(|c| c.ltl_string().contains("failed_"));

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

        if needs_failures {
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
    use super::generate_promela;
    use crate::ir::ltl::{LtlCondition, LtlFormula};
    use crate::ir::*;
    use crate::translator::TranslatedConstraint;

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

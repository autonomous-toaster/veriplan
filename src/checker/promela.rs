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

    // ── Task execution processes (phase-ordered only) ──
    for task in &plan.tasks {
        let av = active_var(&task.id);
        let dv = done_var(&task.id);
        let fv = fail_var(&task.id);

        writeln!(
            s,
            "active proctype task_{}() {{",
            &task.id.replace('.', "_")
        )
        .ok();

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

        // Task body
        writeln!(s, "\t\t{} = 1;\t/* activate */", av).ok();
        writeln!(s, "\t\t{} = 1;\t/* complete */", dv).ok();
        writeln!(s, "\t\t{} = 0;\t/* deactivate */", av).ok();

        // Non-deterministic failure (for conditional constraint exploration)
        writeln!(s, "\t\tif").ok();
        writeln!(s, "\t\t:: {} = 1;", fv).ok();
        writeln!(s, "\t\t:: skip;").ok();
        writeln!(s, "\t\tfi;").ok();

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

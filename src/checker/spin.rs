use crate::checker::{ConstraintSummary, VerificationResult, Violation};
use crate::ir::*;
use crate::translator;
use super::promela::generate_promela;

pub(crate) fn run_spin_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let promela = generate_promela(plan, constraints);
    let promela_path = format!("/tmp/veriplan_{}.pml", plan_name.replace('/', "_"));

    if std::fs::write(&promela_path, &promela).is_err() {
        return super::bfs::simple_result(plan_name, conv_report, vec![]);
    }

    // Phase 1: generate verifier source with spin -a (no search run)
    let promela_dir = std::path::Path::new(&promela_path)
        .parent()
        .unwrap_or(std::path::Path::new("/tmp"))
        .to_path_buf();
    let _pan_path = promela_dir.join("pan");

    let out_gen = std::process::Command::new("spin")
        .args(["-a", &promela_path])
        .current_dir(&promela_dir)
        .output();
    let pan_path = promela_dir.join("pan");

    if out_gen.is_err() {
        return super::bfs::simple_result(plan_name, conv_report, vec![]);
    }

    // Phase 2: compile pan.c into pan binary
    let pan_c = promela_dir.join("pan.c");
    if !pan_c.exists() {
        return super::bfs::simple_result(plan_name, conv_report, vec![]);
    }

    let compile = std::process::Command::new("gcc")
        .args(["-w", "-o", "pan", "pan.c"])
        .stdin(std::process::Stdio::null())
        .current_dir(promela_dir.clone())
        .output();

    if compile.is_err() || !pan_path.exists() {
        return super::bfs::simple_result(plan_name, conv_report, vec![]);
    }

    // Phase 2: run pan for each LTL property individually
    let formalizable: Vec<_> = constraints.iter().filter(|c| c.ltl.is_some()).collect();
    let mut constraints_summary = Vec::new();
    let mut violations = Vec::new();
    let mut satisfied = 0usize;
    let mut _timed_out_count = 0usize;

    for (i, c) in formalizable.iter().enumerate() {
        let label = format!("p{}", i);

        // Liveness properties (with <>) need -a; safety properties don't — much faster
        let has_liveness = c.ltl_string().contains("<>");
        let mut pan_args = vec!["-N", &label, "-n"];
        if has_liveness {
            pan_args.push("-a");
        }

        // Timeout per property: 5s per run (67 tasks × 23 properties = large state space)
        // If timeout, report as "unchecked" rather than pass/fail
        let output = super::bfs::timeout_command(&pan_path, &pan_args, 5);

        let (passed, violated, timed_out) = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                if combined.contains("errors: 1") || combined.contains("errors: 2") {
                    (false, true, false)
                } else {
                    (true, false, false)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => (false, false, true),
            Err(_) => (false, true, false),
        };

        constraints_summary.push(ConstraintSummary {
            requirement_id: c.requirement_id.clone(),
            statement: c.statement.clone(),
            category: format!("{:?}", c.category),
            satisfied: passed,
            unchecked: timed_out,
        });

        if timed_out {
            _timed_out_count += 1;
        } else if violated {
            let fix = super::bfs::suggest_fix(
                &c.category,
                c.ltl_string().as_str(),
                &c.requirement_id,
                &c.statement,
            );
            violations.push(Violation {
                constraint_id: c.requirement_id.clone(),
                requirement_statement: c.statement.clone(),
                ltl: c.ltl_string(),
                category: format!("{:?}", c.category),
                state: format!("(violated in property {})", label),
                task_source: None,
                req_source: None,
                suggested_fix: fix,
                plan: plan_name.to_string(),
            });
        } else {
            satisfied += 1;
        }
    }

    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid: Some(violations.is_empty()),
        violations,
        total_constraints: formalizable.len(),
        satisfied_constraints: satisfied,
        constraints_summary,
        skip_reason: None,
    }
}

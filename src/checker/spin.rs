use super::promela::generate_promela;
use crate::checker::{ConstraintSummary, VerificationResult, Violation};
use crate::ir::*;
use crate::translator;

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

    // Phase 2: run pan once per property, named explicitly. A default pan run evaluates
    // only the first claim, so a plan with several properties would be under-reported.
    let mut constraints_summary = Vec::new();
    let mut violations = Vec::new();
    let mut satisfied = 0usize;
    let mut unmodelled = 0usize;
    let mut unchecked = 0usize;
    let mut claim_index = 0usize;

    for c in constraints {
        let category = format!("{:?}", c.category);
        if !c.modelled() {
            unmodelled += 1;
            constraints_summary.push(ConstraintSummary {
                requirement_id: c.requirement_id.clone(),
                statement: c.statement.clone(),
                category,
                satisfied: false,
                unchecked: false,
                modelled: false,
                unmodelled_reason: c.unmodelled_reason.clone(),
            });
            continue;
        }

        let label = format!("p{}", claim_index);
        claim_index += 1;
        let has_liveness = c.ltl_string().contains("<>");
        let mut pan_args = vec!["-N", &label, "-n"];
        if has_liveness {
            pan_args.push("-a");
        }

        let output = super::bfs::timeout_command(&pan_path, &pan_args, 5);
        let outcome = match output {
            Ok(out) => {
                let combined = format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                pan_outcome(&combined)
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => PanOutcome::Unchecked,
            Err(_) => PanOutcome::Unchecked,
        };

        let (is_satisfied, is_unchecked) = match outcome {
            PanOutcome::Passed => {
                satisfied += 1;
                (true, false)
            }
            PanOutcome::Violated => {
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
                    category: category.clone(),
                    state: format!("(violated in property {})", label),
                    task_source: None,
                    req_source: None,
                    suggested_fix: fix,
                    plan: plan_name.to_string(),
                    kind: crate::ir::kind_of(&category),
                    op: crate::ir::Op::ReplaceBody,
                });
                (false, false)
            }
            PanOutcome::Unchecked | PanOutcome::Empty => {
                unchecked += 1;
                (false, true)
            }
        };

        constraints_summary.push(ConstraintSummary {
            requirement_id: c.requirement_id.clone(),
            statement: c.statement.clone(),
            category,
            satisfied: is_satisfied,
            unchecked: is_unchecked,
            modelled: true,
            unmodelled_reason: None,
        });
    }

    // A verdict that nothing supports is not a verdict. If no constraint was satisfied and
    // none was refuted, the run proved nothing, whatever the engine printed.
    let nothing_proved = satisfied == 0 && violations.is_empty();
    let (valid, skip_reason) = if nothing_proved && !constraints.is_empty() {
        (
            None,
            Some(format!(
                "no constraint was proved or refuted ({} unmodelled, {} unchecked)",
                unmodelled, unchecked
            )),
        )
    } else {
        (Some(violations.is_empty()), None)
    };

    VerificationResult {
        plan_name: plan_name.to_string(),
        phase: "model_check".into(),
        convertible: true,
        convertibility_report: Some(conv_report),
        valid,
        violations,
        total_constraints: constraints.len(),
        satisfied_constraints: satisfied,
        unmodelled_constraints: unmodelled,
        unchecked_constraints: unchecked,
        backend: "spin".into(),
        constraints_summary,
        skip_reason,
    }
}

/// Whether pan's summary reports a refutation. Any non-zero error count counts.
pub(crate) fn pan_found_error(combined: &str) -> bool {
    int_after(combined, "errors:")
        .map(|n| n > 0)
        .unwrap_or(false)
}

/// What a single pan run established.
enum PanOutcome {
    /// The claim held and the search completed.
    Passed,
    /// The engine refuted the claim.
    Violated,
    /// The search did not complete, or explored nothing: no verdict.
    Unchecked,
    /// The engine ran but explored a single state: it checked nothing.
    Empty,
}

fn pan_outcome(combined: &str) -> PanOutcome {
    // 'errors: N' is pan's summary; any non-zero N is a refutation, whatever the count.
    // Matching only 'errors: 1' and 'errors: 2' silently passed every property with three.
    if let Some(n) = int_after(combined, "errors:")
        && n > 0
    {
        return PanOutcome::Violated;
    }
    if combined.contains("Search not completed") || combined.contains("depth limit") {
        return PanOutcome::Unchecked;
    }
    let states = int_before(combined, " states, stored").unwrap_or(0);
    let transitions = int_before(combined, " transitions").unwrap_or(0);
    if states <= 1 && transitions == 0 {
        return PanOutcome::Empty;
    }
    PanOutcome::Passed
}

/// Parse the first integer following the marker.
fn int_after(hay: &str, marker: &str) -> Option<usize> {
    let at = hay.find(marker)? + marker.len();
    let digits: String = hay[at..]
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Parse the integer immediately preceding the marker.
fn int_before(hay: &str, marker: &str) -> Option<usize> {
    let at = hay.find(marker)?;
    let digits: String = hay[..at]
        .trim_end()
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse().ok()
}

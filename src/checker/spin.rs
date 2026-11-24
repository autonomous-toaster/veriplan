use std::time::Duration;

use super::promela::generate_promela;
use crate::checker::{ConstraintSummary, VerificationResult, Violation};
use crate::ir::*;
use crate::translator;

/// The wall-clock allowance given to one claim's external search.
///
/// pan has no state ceiling of its own, so this is the only bound the external backend has. It is
/// reported with the claim it stopped, and under its own reason: a run that ran out of time is not
/// a run that exhausted a budget.
const PAN_WALL_CLOCK: Duration = Duration::from_secs(5);

pub(crate) fn run_spin_check(
    plan: &PlanIR,
    plan_name: &str,
    constraints: &[translator::TranslatedConstraint],
    conv_report: ConvertibilityReport,
) -> VerificationResult {
    let Some(pan_path) = prepare_pan(plan, constraints, plan_name) else {
        return super::bfs::simple_result(plan_name, conv_report, vec![]);
    };

    // Phase 2: run pan once per property, named explicitly. A default pan run evaluates
    // only the first claim, so a plan with several properties would be under-reported.
    let mut constraints_summary = Vec::new();
    let mut violations = Vec::new();
    let mut satisfied = 0usize;
    let mut unmodelled = 0usize;
    let mut historical = 0usize;
    let mut unchecked = 0usize;
    let mut claim_index = 0usize;

    for c in constraints {
        let category = format!("{:?}", c.category);
        if !c.modelled() {
            if c.historical() {
                historical += 1;
            } else {
                unmodelled += 1;
            }
            constraints_summary.push(unmodelled_summary(c, category));
            continue;
        }

        let label = format!("p{}", claim_index);
        claim_index += 1;
        let outcome = run_one_claim(&pan_path, &label, c.ltl_string().contains("<>"));

        let (is_satisfied, unchecked_reason) = claim_status_into(
            outcome,
            c,
            plan_name,
            &label,
            &category,
            &mut satisfied,
            &mut unchecked,
            &mut violations,
        );

        constraints_summary.push(ConstraintSummary {
            requirement_id: c.requirement_id.clone(),
            statement: c.statement.clone(),
            category,
            satisfied: is_satisfied,
            unchecked: unchecked_reason.is_some(),
            modelled: true,
            unchecked_reason,
            states_used: None,
            states_allowed: None,
            historical: false,
            historical_owner: None,
            unmodelled_reason: None,
        });
    }

    // A verdict that nothing supports is not a verdict. If no constraint was satisfied and
    // none was refuted, the run proved nothing, whatever the engine printed.
    let (valid, skip_reason) = verdict(satisfied, unmodelled, unchecked, &violations);

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
        historical_constraints: historical,
        backend: "spin".into(),
        constraints_summary,
        skip_reason,
    }
}

/// Write the model, generate the verifier with `spin -a` and compile it, returning the `pan` binary.
///
/// None means the toolchain could not produce a checker, which is the same "no verdict" the caller
/// already reports for a missing engine.
fn prepare_pan(
    plan: &PlanIR,
    constraints: &[translator::TranslatedConstraint],
    plan_name: &str,
) -> Option<std::path::PathBuf> {
    let promela = generate_promela(plan, constraints);
    let promela_path = format!("/tmp/veriplan_{}.pml", plan_name.replace('/', "_"));
    std::fs::write(&promela_path, &promela).ok()?;

    let dir = std::path::Path::new(&promela_path)
        .parent()
        .unwrap_or(std::path::Path::new("/tmp"))
        .to_path_buf();
    std::process::Command::new("spin")
        .args(["-a", &promela_path])
        .current_dir(&dir)
        .output()
        .ok()?;

    if !dir.join("pan.c").exists() {
        return None;
    }
    std::process::Command::new("gcc")
        .args(["-w", "-o", "pan", "pan.c"])
        .stdin(std::process::Stdio::null())
        .current_dir(dir.clone())
        .output()
        .ok()?;

    let pan = dir.join("pan");
    pan.exists().then_some(pan)
}

/// A verdict that nothing supports is not a verdict: when no constraint was satisfied and none was
/// refuted, the run proved nothing whatever the engine printed.
fn verdict(
    satisfied: usize,
    unmodelled: usize,
    unchecked: usize,
    violations: &[Violation],
) -> (Option<bool>, Option<String>) {
    if satisfied == 0 && violations.is_empty() {
        return (
            None,
            Some(format!(
                "no constraint was proved or refuted ({unmodelled} unmodelled, {unchecked} unchecked)"
            )),
        );
    }
    (Some(violations.is_empty()), None)
}

/// The summary of a constraint that carries no formula, counted as unmodelled or as historical.
fn unmodelled_summary(c: &translator::TranslatedConstraint, category: String) -> ConstraintSummary {
    ConstraintSummary {
        requirement_id: c.requirement_id.clone(),
        statement: c.statement.clone(),
        category,
        satisfied: false,
        unchecked: false,
        modelled: false,
        unchecked_reason: None,
        states_used: None,
        states_allowed: None,
        historical: c.historical.is_some(),
        historical_owner: c.historical.clone(),
        unmodelled_reason: c.unmodelled_reason.clone(),
    }
}

/// Record what a claim's run established and say whether it counted as satisfied or unchecked.
#[allow(clippy::too_many_arguments)]
fn claim_status_into(
    outcome: (PanOutcome, Duration),
    c: &translator::TranslatedConstraint,
    plan_name: &str,
    label: &str,
    category: &str,
    satisfied: &mut usize,
    unchecked: &mut usize,
    violations: &mut Vec<Violation>,
) -> (bool, Option<String>) {
    let (outcome, elapsed) = outcome;
    match outcome {
        PanOutcome::Passed => {
            *satisfied += 1;
            (true, None)
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
                category: category.to_string(),
                state: format!("(violated in property {label})"),
                task_source: None,
                req_source: None,
                suggested_fix: fix,
                plan: plan_name.to_string(),
                kind: crate::ir::kind_of(category),
                op: crate::ir::Op::ReplaceBody,
            });
            (false, None)
        }
        // A claim the external backend did not decide is unchecked, and says why. It used to say
        // "state_budget", which is the tag for a budget an engine consulted: pan has none.
        PanOutcome::Unchecked(stop) => {
            *unchecked += 1;
            (false, Some(stop.reason(elapsed)))
        }
        PanOutcome::Empty => {
            *unchecked += 1;
            (false, Some(PanStop::NothingExplored.reason(elapsed)))
        }
    }
}

/// Run one claim and say what the run established.
///
/// A liveness claim needs the acceptance-cycle search; a safety claim does not, and asking for it
/// would cost the whole state space for nothing.
fn run_one_claim(
    pan_path: &std::path::Path,
    label: &str,
    has_liveness: bool,
) -> (PanOutcome, Duration) {
    let mut args = vec!["-N", label, "-n"];
    if has_liveness {
        args.push("-a");
    }
    let started = std::time::Instant::now();
    let outcome = match super::bfs::timeout_command(pan_path, &args, PAN_WALL_CLOCK.as_secs()) {
        Ok(out) => {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            pan_outcome(&combined)
        }
        Err(_) => PanOutcome::Unchecked(PanStop::WallClock {
            allowed: PAN_WALL_CLOCK,
        }),
    };
    (outcome, started.elapsed())
}

/// Whether pan's summary reports a refutation. Any non-zero error count counts.
pub(crate) fn pan_found_error(combined: &str) -> bool {
    int_after(combined, "errors:")
        .map(|n| n > 0)
        .unwrap_or(false)
}

/// Why a claim got no verdict from the external backend.
///
/// The wall clock and the state budget are different facts. pan has no state budget, so a claim it
/// stopped said "state_budget" and named a budget that nothing consulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanStop {
    /// The search stopped at the wall-clock allowance.
    WallClock { allowed: Duration },
    /// The engine reported a search that did not complete.
    NotCompleted,
    /// The engine ran but explored a single state: it checked nothing.
    NothingExplored,
}

impl PanStop {
    /// The reason recorded against the claim, including the time it was allowed and the time it took.
    fn reason(self, used: Duration) -> String {
        match self {
            PanStop::WallClock { allowed } => format!(
                "wallclock: stopped at the {}s budget after {:.2}s",
                allowed.as_secs(),
                used.as_secs_f64()
            ),
            PanStop::NotCompleted => {
                format!(
                    "not_completed: the search did not complete ({:.2}s)",
                    used.as_secs_f64()
                )
            }
            PanStop::NothingExplored => format!(
                "not_completed: the search explored one state and no transitions ({:.2}s)",
                used.as_secs_f64()
            ),
        }
    }
}

/// What a single pan run established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanOutcome {
    /// The claim held and the search completed.
    Passed,
    /// The engine refuted the claim.
    Violated,
    /// The search produced no verdict, and this is why.
    Unchecked(PanStop),
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
        return PanOutcome::Unchecked(PanStop::NotCompleted);
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

#[cfg(test)]
mod tests {
    // Tests fail by panicking; the crate denies these three lints for code that ships.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// Every count above zero is a refutation, not only the ones a hand-written match would name.
    #[test]
    fn any_error_count_refutes() {
        for n in [1, 2, 3, 17] {
            let text = format!("pan: wrote foo.trail\nerrors: {n}\n");
            assert!(
                matches!(pan_outcome(&text), PanOutcome::Violated),
                "errors: {n}"
            );
        }
    }

    /// A run that stopped early examined less than the state space, so it decides nothing.
    #[test]
    fn an_incomplete_search_is_unchecked() {
        for marker in ["Search not completed", "depth limit reached"] {
            assert!(
                matches!(
                    pan_outcome(marker),
                    PanOutcome::Unchecked(PanStop::NotCompleted)
                ),
                "{marker}"
            );
        }
    }

    /// A single state with no transitions is pan running without checking anything.
    #[test]
    fn a_single_state_with_no_transitions_is_empty() {
        let text = "errors: 0\n\t1 states, stored\n\t     0 transitions\n";
        assert!(matches!(pan_outcome(text), PanOutcome::Empty));
    }

    /// A stopped search and an exhausted budget are different facts, and only one of them is a
    /// budget. pan has none, so no reason rendered from a pan outcome may name one.
    #[test]
    fn a_stopped_search_names_its_own_cause_and_its_own_cost() {
        let allowed = Duration::from_secs(5);
        let used = Duration::from_millis(5_010);
        let wallclock = PanStop::WallClock { allowed }.reason(used);
        let not_completed = PanStop::NotCompleted.reason(used);
        let nothing = PanStop::NothingExplored.reason(used);

        assert!(wallclock.starts_with("wallclock:"), "{wallclock}");
        assert!(wallclock.contains("5s"), "{wallclock}");
        assert!(wallclock.contains("5.01s"), "{wallclock}");
        for reason in [&wallclock, &not_completed, &nothing] {
            assert!(
                !reason.contains("state_budget"),
                "a pan outcome named a budget nothing consulted: {reason}"
            );
        }
        assert!(
            not_completed.starts_with("not_completed:"),
            "{not_completed}"
        );
        assert!(nothing.starts_with("not_completed:"), "{nothing}");
    }

    /// Otherwise the search closed and the claim held.
    #[test]
    fn a_closed_search_passes() {
        let text = "errors: 0\n\t8 states, stored\n\t    11 transitions\n";
        assert!(matches!(pan_outcome(text), PanOutcome::Passed));
    }
}

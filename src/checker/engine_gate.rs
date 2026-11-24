#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // Tests fail by panicking; the crate denies these for code that ships.

//! The in-process engine's gate.
//!
//! These tests measure the engine, not the model, so they build their Promela as source rather than
//! through the generator: a change to the generator must not move a number that is about the engine.
//! They live in their own file for the same reason the model's state-space test is measured with
//! `pan` - a regression in one must not be able to fail a test about the other.
//!
//! Which fixture measures what: a chain measures the state projection, because the flag it drops is
//! read by nobody; a fan-out of independent processes measures interleaving, which no projection can
//! remove because the program counters are always read. These fixtures are chained on purpose.

use crate::checker::spin_rs::{DEFAULT_STATE_BUDGET, state_budget_from};

/// A budget the caller set is honoured or refused, never replaced. Zero means "do not run",
/// and a typo is a caller error; both used to become 200 000 with no word to the caller.
#[test]
fn a_budget_the_caller_set_is_honoured_or_refused() {
    assert_eq!(state_budget_from(None).ok(), Some(DEFAULT_STATE_BUDGET));
    assert_eq!(state_budget_from(Some("5000")).ok(), Some(5_000));
    assert_eq!(state_budget_from(Some(" 5000 ")).ok(), Some(5_000));
    assert!(
        state_budget_from(Some("0")).is_err(),
        "zero must be refused"
    );
    assert!(
        state_budget_from(Some("abc")).is_err(),
        "a typo must be refused"
    );
    assert!(
        state_budget_from(Some("-1")).is_err(),
        "a negative is not a state count"
    );
}

/// A chain of `tasks` proctypes. The first `branched` carry a nondeterministic failure flag
/// and a branch; one safety claim names the first flag and holds in every reachable state, so
/// the space closes. Built as source rather than through the generator: this measures the
/// engine, and a change to the generator must not move the number.
fn chain(tasks: usize, branched: usize) -> String {
    let mut s = String::new();
    for i in 1..=tasks {
        s.push_str(&format!("bit active_t{i} = 0;\nbit done_t{i} = 0;\n"));
    }
    for i in 1..=branched {
        s.push_str(&format!("bit failed_t{i} = 0;\n"));
    }
    for i in 1..=tasks {
        let guard = if i == 1 {
            "(1)".to_string()
        } else {
            format!("(done_t{} == 1)", i - 1)
        };
        s.push_str(&format!(
                "active proctype task_{i}() {{\n\tdo\n\t:: {guard} ->\n\t\tactive_t{i} = 1;\n\t\tactive_t{i} = 0;\n"
            ));
        if i <= branched {
            s.push_str(&format!(
                "\t\tif\n\t\t:: failed_t{i} = 1;\n\t\t:: skip;\n\t\tfi;\n"
            ));
        }
        s.push_str(&format!("\t\tdone_t{i} = 1;\n\t\tbreak\n\tod\n}}\n"));
    }
    s.push_str("ltl p0 { [] ( failed_t1 -> failed_t1 ) }\n");
    s
}

/// States per task a linear chain may use. Measured: a chain carrying one branch uses ten.
const STATES_PER_TASK_CEILING: usize = 20;

/// The smallest task count whose state count has stopped growing linearly, if any.
///
/// A linear chain spends a fixed number of states per task; a chain that branches on every task
/// spends a factor of two per task. Naming the task count is what makes the failure message
/// useful: "not linear" alone does not say where it stopped being linear.
fn first_non_linear(counts: &[(usize, usize)]) -> Option<usize> {
    counts
        .iter()
        .find(|(tasks, states)| states > &(STATES_PER_TASK_CEILING * tasks))
        .map(|(tasks, _)| *tasks)
}

/// The in-process engine's bound, on a shape this repository generates.
///
/// Both numbers are pinned because they fail for different reasons: the state count catches a
/// claim-relative projection that stopped projecting, and the elapsed time catches a per-state
/// cost that regressed. A single assertion would not say which.
#[test]
fn the_in_process_engine_stays_under_its_recorded_bound() {
    let model = chain(20, 1);
    let started = std::time::Instant::now();
    let result = spin_rs::verify(&model).expect("the twenty-task chain must be checkable");
    let elapsed = started.elapsed();

    assert!(
        result.violations.is_empty(),
        "the gate fixture must hold in every reachable state, or the space never closes"
    );
    // Measured on this fixture: 202 states, 0.15 s in a debug build.
    // The count is deterministic, so it is asserted exactly. An upper bound near the measured value
    // is the mistake this gate exists to avoid: a ceiling of 400 against 202 would pass a regression
    // to every value in between, and the broken case - a branch on every task - is 124 at eight
    // tasks and 2 806 at eight with no projection, nowhere near 400.
    assert_eq!(
        result.states_stored,
        202,
        "twenty tasks with one branch explored {} states in {:.2}s; the pinned engine explored 202, \
         and no projection changes this fixture because its guards read every done_ field",
        result.states_stored,
        elapsed.as_secs_f64()
    );
    // The wall clock is an order-of-magnitude guard, not a canary: 0.15 s measured against a 5 s
    // bound cannot fail for anything except a catastrophe. The count above is what catches a
    // regression; this catches the engine ceasing to terminate in a human lifetime.

    assert!(
        elapsed <= std::time::Duration::from_secs(5),
        "twenty tasks with one branch took {:.2}s for {} states",
        elapsed.as_secs_f64(),
        result.states_stored
    );
}

/// The liveness product, measured on the pinned engine, per task count.
///
/// Columns: tasks, states with the failure flag and a claim that names the flag, states without
/// either. Recorded as exact numbers rather than as a ceiling, because a ceiling wide enough to
/// be comfortable cannot fail: an upper bound of 4 000 on a measured 2 552 catches only a
/// regression bigger than 1.6x, and would pass whether or not the engine ever projects the state
/// of a liveness claim.
const LIVENESS_PRODUCT: &[(usize, usize, usize)] = &[(6, 632, 316), (8, 2_552, 1_276)];

/// The same chain under a claim that carries a liveness operator.
///
/// This exists because the safety gate above does not see the liveness path at all. The pinned
/// engine gets no state projection for a model whose only claims are liveness claims - its
/// reduction scopes skip every formula that is not a safety formula - so this fixture pays the
/// full product space where the safety fixture pays ten states per task. The recorded curve is
/// 632 / 2 552 / 10 232 / 40 952 at 6 / 8 / 10 / 12 tasks: a factor of four per two tasks.
///
/// The pair asserts the defect in one number. Dropping the invisible bit halves the product
/// exactly, at every task count, because nothing projects a field the claim cannot see. So while
/// the third column is half the second, the branch is still being paid for.
///
/// A change in any of these numbers is the signal, in either direction. A decrease means the
/// engine now projects liveness claims and the table must be lowered to match; an increase is a
/// regression. A ceiling nobody lowers is how a 100x hole becomes invisible.
#[test]
fn the_liveness_path_is_measured_too() {
    for &(tasks, with_flag, without_flag) in LIVENESS_PRODUCT {
        let measured = spin_rs::verify(&chain_with_liveness(tasks))
            .expect("the liveness chain must be checkable");
        assert!(
            measured.violations.is_empty(),
            "the fixture must hold in every reachable state, or the product never closes"
        );
        assert_eq!(
            measured.states_stored, with_flag,
            "the liveness product for {tasks} tasks changed from {with_flag} to {}; a fall means \
                 the engine now projects this claim and the table needs lowering, a rise means a \
                 regression",
            measured.states_stored
        );

        let plain = spin_rs::verify(&chain_with_liveness_without_the_flag(tasks))
            .expect("the chain without the flag must be checkable");
        assert_eq!(
            plain.states_stored, without_flag,
            "the liveness product without the invisible bit changed from {without_flag} to {}",
            plain.states_stored
        );
        assert_eq!(
            measured.states_stored,
            plain.states_stored * 2,
            "the invisible bit no longer costs exactly double; if the engine started projecting \
                 liveness claims both rows should collapse and the table needs lowering"
        );
    }
}

/// The gate fixture under a claim that carries a liveness operator.
fn chain_with_liveness(tasks: usize) -> String {
    with_claim(
        chain(tasks, 1),
        &format!("ltl p0 {{ [] ( failed_t1 -> <> active_t{tasks} ) }}"),
    )
}

/// The same chain with no failure flag at all, and a claim that names only visible bits.
fn chain_with_liveness_without_the_flag(tasks: usize) -> String {
    with_claim(
        chain(tasks, 0),
        &format!("ltl p0 {{ [] ( done_t1 -> <> active_t{tasks} ) }}"),
    )
}

/// Replace the safety claim of a generated chain with the given claim.
fn with_claim(mut source: String, claim: &str) -> String {
    source.truncate(source.rfind("ltl p0").unwrap_or(source.len()));
    source.push_str(claim);
    source.push('\n');
    source
}
/// The safety canary: a branch on every task, recorded exactly.
///
/// The states the pinned engine spends when a claim names one flag and every task carries a
/// branch. Engines that project claim-visible state take these to 92 and 124, measured on two later
/// tags; neither is pinned. Each also stopped refuting a construct this pin refutes - an `unless`
/// handler and a do-loop assertion - and the construct corpus refused both. A projection that loses
/// a refutation is worth less than a count that stays high.
///
/// A fall here means a pin arrived that keeps the verdicts, and the table is then lowered
/// deliberately; a rise is a regression.
const BRANCHING_CANARY: &[(usize, usize)] = &[(6, 694), (8, 2_806)];

/// The detector behind the gate reads a curve, so it is tested on recorded curves rather than on
/// a live model.
///
/// Testing it against the live exponential fixture would stop testing the detector the moment
/// the engine fixed the fixture: the assertion would fail because the model became linear, which
/// looks exactly like a regression in the detector. The live numbers belong in the tables, where
/// a change is the signal.
#[test]
fn the_gate_fails_a_model_that_branches_on_every_task() {
    let linear = [(8usize, 82usize), (12, 122), (16, 162), (20, 202)];
    assert_eq!(
        first_non_linear(&linear),
        None,
        "a chain carrying one branch must stay linear"
    );

    let branching = [(6usize, 694usize), (8, 2_806)];
    assert_eq!(
        first_non_linear(&branching),
        Some(6),
        "a branch on every task must be caught at the smallest count"
    );
}

/// The safety canary is measured, not quoted: the table above must describe what the pinned
/// engine actually does.
#[test]
fn the_branching_canary_matches_the_pinned_engine() {
    for &(tasks, states) in BRANCHING_CANARY {
        let measured = spin_rs::verify(&chain(tasks, tasks))
            .expect("the branching chain must be checkable")
            .states_stored;
        assert_eq!(
            measured, states,
            "the branching canary for {tasks} tasks changed from {states} to {measured}; a fall \
                 means the engine now projects this claim and the table needs lowering, a rise is a \
                 regression"
        );
        // On this pin the fixture is exponential, and that is the defect the canary records. When a
        // pin arrives that projects this claim and keeps the verdicts, the count above falls and this
        // becomes None; the assertion is then inverted as part of lowering the table.
        assert_eq!(
            first_non_linear(&[(tasks, measured)]),
            Some(tasks),
            "the canary for {tasks} tasks is no longer exponential, so it no longer records the defect"
        );
    }
}

/// Two flags present, and a claim that cannot see either.
///
/// This is the only shape in which the engine's liveness projection pays, and it is the shape
/// veriplan actually produces: a plan with several claims where some name a failure flag and the
/// rest cannot see it. A single-claim model cannot show it, because the generator emits a flag only
/// when a claim names that flag, so "the bits exist" and "the claim can see them" collapse into one
/// condition. Recorded because the single-claim liveness fixture above can sit still forever and
/// still be measuring nothing.
const BLIND_CLAIM_PRODUCT: &[(usize, usize)] = &[(8, 154), (12, 234)];

/// A chain whose first two tasks carry a failure flag, under a claim that names neither.
fn chain_with_two_flags_and_a_blind_claim(tasks: usize) -> String {
    with_claim(chain(tasks, 2), "ltl p0 { [] ( done_t1 -> done_t1 ) }")
}

#[test]
fn a_claim_blind_to_a_flag_another_claim_names_is_projected() {
    for &(tasks, states) in BLIND_CLAIM_PRODUCT {
        let measured = spin_rs::verify(&chain_with_two_flags_and_a_blind_claim(tasks))
            .expect("the blind-claim chain must be checkable")
            .states_stored;
        assert_eq!(
            measured, states,
            "the blind-claim product for {tasks} tasks changed from {states} to {measured}; at \
             spin-rs v0.5.35 the same fixture explored 154 and 234, so a return to those numbers \
             means the engine stopped projecting a claim that cannot see the flag"
        );
    }
}

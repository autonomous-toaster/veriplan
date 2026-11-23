//! Differential guard over Promela constructs this tool's emitter never produces.
//!
//! Every model here was checked against `pan` by hand, and the answer is recorded beside it. The engine
//! may refuse a construct or decline a claim - both are "no verdict" - but it may not decide a model and
//! disagree with pan, and it may not refute while declining.
//!
//! The value is that it stays meaningful when the engine grows: today every construct is refused or
//! declined, so the test passes by finding no verdict. When one is implemented, the same table starts
//! asserting the verdict, which is the moment a wrong answer would otherwise slip in.

use std::path::Path;

/// `(file, pan's error count or None when pan cannot build the model)`.
///
/// The counts were taken by running `spin -a` and `pan -m50000` on each file.
const MODELS: &[(&str, Option<usize>)] = &[
    ("priority.pml", Some(0)),
    ("priority_none.pml", Some(1)),
    // A never claim over a model that terminates: `never { skip }` is refuted by pan, and the same
    // model with an arm no reachable state satisfies is not. Only the claim body differs, which is what
    // separates "acceptance condition" from "a claim is present".
    ("never.pml", Some(1)),
    ("never_unreachable.pml", Some(0)),
    // spin's own automaton for <> ( got ), embedded verbatim. spin -f emits the negation automaton, so
    // the refuting file is the one where the property holds. This pair exercises a claim's *assert*,
    // not an acceptance cycle: pan's refutation is "assertion violated !(got)" and the accept state is
    // unreachable behind a do-loop that never terminates. The cycle pair is below.
    ("never_accept_holds.pml", Some(0)),
    ("never_accept_refutes.pml", Some(1)),
    // The acceptance-cycle pair, and the only rows here that test a cycle. These expectations are pan's
    // answers under -a: plain pan reports 0 for both, because without -a it does not look for acceptance
    // cycles at all - so a reader who checks them with plain pan would conclude the pair is vacuous.
    ("never_cycle_accept.pml", Some(1)),
    ("never_cycle_unreachable.pml", Some(0)),
    // The two shapes the engine refuses with a design rather than an unknown. Both are oracles for the
    // next step: pan decides them, and neither is reachable by anything this tool emits.
    ("poll_rendezvous.pml", Some(0)),
    ("poll_no_sender.pml", Some(1)), // pan -a; plain pan reports 0 for both files in this pair
    ("unless_multi_handler.pml", Some(1)),
    // The handler armed at the body's second step, and the same body with no handler. The first pair of
    // unless models only fixed the boundary at the first step, so a counter off by one later would pass it.
    ("unless_second_step.pml", Some(0)),
    ("unless_second_step_nohandler.pml", Some(1)),
    // Same arming, two body shapes: a block that has already finished cannot be preempted, a loop is
    // re-checked and the handler fires. pan 0 and 1.
    ("unless_last_step_block.pml", Some(0)),
    ("unless_last_step_loop.pml", Some(1)),
    // Which matching message a random receive takes: first from the head, or the last one.
    ("randrecv_first_match.pml", Some(0)),
    // A multi-field message round trip, deciding since v0.5.17.
    ("twofield_message.pml", Some(0)),
    // The oracle for run(): the process count is named while a child is alive, and the control claims one
    // process instead of two. Both are refused today, so neither asserts yet.
    // Arrays and indexed process instantiation decide today. They are here because a histogram bucket
    // labelled "array" turned out to name the item containing a failure rather than the construct, and a
    // regression in either would otherwise be visible only as a refused shipped example.
    // Two declarators in one statement inside a body: pan accepts, the parser stops at the comma.
    // mtype numbering: values start at 1 and run in reverse declaration order, so a = 3 for { a, b, c }.
    // A process looping through a label whose two assignments a second process can observe between. This
    // pair did NOT reproduce the reduction unsoundness found in ex_3c - both members decide 1 at v0.5.23 and
    // at v0.5.24, though the explored-state counts differ - so it is a coarse guard on the verdict only.
    // Parameter binding: a run() argument must reach the callee, for a value and for a channel. All three
    // are refused today, so they assert the day parameterised proctypes and run land together.
    ("param_bound.pml", Some(0)),
    ("param_bound_control.pml", Some(1)),
    ("param_chan.pml", Some(0)),
    ("goto_loop_partial.pml", Some(1)),
    ("goto_loop_control.pml", Some(1)),
    ("mtype_numbering.pml", Some(0)),
    ("mtype_numbering_control.pml", Some(1)),
    // Refused since v0.5.21 - an empty state space is an error whatever claims are present - so it no
    // longer reports a silent pass. pan refutes it, so the row asserts when the initialiser is supported.
    ("mtype_init_silent_pass.pml", Some(1)),
    ("multi_declarator_body.pml", Some(0)),
    ("array_element.pml", Some(0)),
    ("array_element_control.pml", Some(1)),
    ("active_n_proctype.pml", Some(0)),
    ("nrpr_child_alive.pml", Some(0)),
    ("nrpr_child_count_off.pml", Some(1)),
    ("evalrecv_waits.pml", Some(1)),
    ("live_cmp_atom.pml", Some(0)),
    // pan disables the model's own terminal state as an error whenever a claim is present, and with any
    // claim the engine now does the same: this fixture was a known disagreement until v0.5.13.
    ("blocked_true_claim.pml", Some(0)),
    ("unless.pml", Some(0)),
    ("unless_refutes.pml", Some(1)),
    // The handler is armed by the first statement of the body, so an encoding that checks the escape
    // only at the loop top refutes a claim pan accepts. The *_toptop file is that wrong encoding,
    // written out, and pan refutes it - which is what makes the pair discriminating rather than two
    // more models that happen to agree.
    ("unless_midbody.pml", Some(0)),
    ("unless_midbody_toptop.pml", Some(1)),
    ("rendezvous.pml", Some(0)),
    ("rendezvous_holds.pml", Some(0)),
    ("rendezvous_refutes.pml", Some(1)),
    ("rendezvous_order.pml", Some(0)),
    ("run.pml", Some(0)),
    ("assert_not_bool_false.pml", Some(0)),
    ("assert_bool_equals_zero.pml", Some(0)),
    ("assign_bool_literal.pml", Some(1)),
    ("assign_bool_to_bool.pml", Some(0)),
];

/// Models where the engine disagrees with pan today. Listed rather than deleted so the disagreement runs
/// on demand and cannot be forgotten; kept out of the asserting table so the default suite does not fail
/// for a defect this project cannot fix.
/// Empty since v0.5.13 fixed the last one. The machinery stays because the next divergence needs a home
/// that runs, not a comment that rots.
const KNOWN_DISAGREEMENTS: &[(&str, usize)] = &[];

/// Keeps the disagreements above alive. Run with `cargo test -- --ignored`: it passes while each still
/// disagrees with pan and fails the moment one is fixed, with the row to move. A list nobody runs is a
/// list that outlives its reasons.
#[test]
#[ignore = "known engine disagreements with pan; run with --ignored to check whether they are fixed"]
fn known_disagreements_still_disagree() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/constructs");
    for (file, pan_errors) in KNOWN_DISAGREEMENTS {
        let src = std::fs::read_to_string(dir.join(file)).unwrap_or_default();
        if let Ok(result) = spin_rs::verify(&src) {
            assert_ne!(
                result.violations.len(),
                *pan_errors,
                "{file}: the engine now agrees with pan - move this row into the asserting table"
            );
        }
    }
}

/// A fixture without a row is a model nobody checks, which is how a guard quietly stops covering what
/// it was built for. Every file in the directory must carry pan's answer.
#[test]
fn every_fixture_has_a_recorded_pan_verdict() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/constructs");
    let mut on_disk: Vec<String> = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("cannot read fixture directory {dir:?}: {e}");
            return;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".pml") {
            on_disk.push(name);
        }
    }
    on_disk.sort();
    let mut listed: Vec<String> = MODELS
        .iter()
        .map(|(f, _)| (*f).to_string())
        .chain(KNOWN_DISAGREEMENTS.iter().map(|(f, _)| (*f).to_string()))
        .collect();
    listed.sort();
    assert_eq!(
        on_disk, listed,
        "a fixture and the table of pan verdicts disagree about which models exist"
    );
}

#[test]
fn a_construct_is_never_decided_against_pan() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/constructs");
    let mut decided = 0usize;
    let mut no_verdict = 0usize;

    for (file, pan_errors) in MODELS {
        let src = match std::fs::read_to_string(dir.join(file)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{file}: cannot read fixture: {e}");
                continue;
            }
        };
        // Every entry records pan's answer; one without it is a table mistake, not a verdict.
        let Some(pan_errors) = *pan_errors else {
            eprintln!("{file}: no pan verdict recorded, skipping");
            continue;
        };

        match spin_rs::verify(&src) {
            // Refused outright: no verdict, which is what the engine promises for these constructs.
            Err(_) => no_verdict += 1,
            Ok(result) => {
                // A violation attributed to the whole model is a deadlock report. pan disables invalid
                // end state checking whenever a claim is present, which for this tool is always, so a
                // model-level violation is never a pan disagreement - it is a different subject.
                let model_violations = result
                    .violations
                    .iter()
                    .filter(|v| v.property_name == "model")
                    .count();
                assert_eq!(
                    model_violations, 0,
                    "{file}: refuted the model itself (deadlock); pan reports none under a claim"
                );

                let declined = result.unverified.iter().any(|u| u.property_name != "model");
                if declined {
                    // No claim verdict. Declining must not come with a claim refutation: the same
                    // claim cannot be both undecided and refuted.
                    assert_eq!(
                        result.violations.len(),
                        0,
                        "{file}: declined a claim and also refuted it"
                    );
                    no_verdict += 1;
                } else {
                    decided += 1;
                    assert_eq!(
                        result.violations.len(),
                        pan_errors,
                        "{file}: decided the model and disagreed with pan"
                    );
                }
            }
        }
    }

    // Both outcomes are acceptable; what is not acceptable is silence about which one happened.
    assert_eq!(
        decided + no_verdict,
        MODELS.len(),
        "every model must be classified"
    );
}

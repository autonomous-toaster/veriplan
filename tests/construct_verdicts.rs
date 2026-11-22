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
    ("never.pml", Some(1)),
    ("unless.pml", Some(0)),
    ("unless_refutes.pml", Some(1)),
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
                if !result.unverified.is_empty() {
                    // Declined some claim. That is no verdict, but declining must not come with a
                    // refutation: a model cannot be both undecided and refuted.
                    assert_eq!(
                        result.errors, 0,
                        "{file}: declined a claim and also refuted it"
                    );
                    no_verdict += 1;
                } else {
                    decided += 1;
                    assert_eq!(
                        result.errors, pan_errors,
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

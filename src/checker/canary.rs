//! Calibration canary.
//!
//! Fixed claims whose verdicts are known, run through the selected backend before any plan result
//! is trusted. Three shapes, each with an expected answer:
//!
//!   p0  [] ( !(ca && cb) )              false  - both flags can be 1
//!   p1  [] ( cf -> <> (ca && !(ca)) )   false  - liveness in the shape veriplan emits, never true.
//!                                           cf is set only inside an if/fi branch, the way every
//!                                           failure flag veriplan generates is, so this claim also
//!                                           requires that branch to be reachable at all.
//!   p2  [] ( ca -> <> (ca) )            TRUE   - the same shape, satisfied
//!
//! p2 is the one that catches a checker which fabricates a violation when it cannot decide a formula:
//! a backend that only ever answers 'refuted' or 'clean' for false claims can still be wrong in the
//! direction that makes every conditional requirement look violated. An engine that reports p2
//! refuted, or leaves any claim undecided, cannot certify a plan.

use super::CheckerBackend;
use std::collections::HashSet;

/// The canary model. Kept tiny so the check costs milliseconds.
pub const CANARY_MODEL: &str = r#"
bool ca = 0;
bool cb = 0;
bool cf = 0;
active proctype CanaryA() { do :: (1) -> ca = 1; break od }
active proctype CanaryB() { do :: (1) -> cb = 1; if :: cf = 1; :: skip; fi; break od }
ltl p0 { [] ( !(ca && cb) ) }
ltl p1 { [] ( cf -> <> (ca && !(ca)) ) }
ltl p2 { [] ( ca -> <> (ca) ) }
"#;

struct Claim {
    label: &'static str,
    liveness: bool,
    must_be_refuted: bool,
}

const CANARY_CLAIMS: [Claim; 3] = [
    Claim {
        label: "p0",
        liveness: false,
        must_be_refuted: true,
    },
    Claim {
        label: "p1",
        liveness: true,
        must_be_refuted: true,
    },
    Claim {
        label: "p2",
        liveness: true,
        must_be_refuted: false,
    },
];

/// Run the canary through a backend. Ok(true) when every claim got its expected verdict.
pub(crate) fn calibrated(backend: CheckerBackend) -> Result<bool, String> {
    match backend {
        CheckerBackend::SpinRs => {
            let result = spin_rs::verify(CANARY_MODEL)
                .map_err(|e| format!("canary could not run on spin-rs: {e}"))?;
            let refuted: HashSet<&str> = result
                .violations
                .iter()
                .map(|v| v.property_name.as_str())
                .collect();
            let undecided: HashSet<&str> = result
                .unverified
                .iter()
                .map(|u| u.property_name.as_str())
                .collect();
            if undecided.contains("model") {
                return Ok(false);
            }
            Ok(CANARY_CLAIMS.iter().all(|claim| {
                !undecided.contains(claim.label)
                    && refuted.contains(claim.label) == claim.must_be_refuted
            }))
        }
        CheckerBackend::Spin => calibrated_by_spin(),
    }
}

fn calibrated_by_spin() -> Result<bool, String> {
    let dir = std::path::Path::new("/tmp/veriplan_canary");
    std::fs::create_dir_all(dir).map_err(|e| format!("canary directory: {e}"))?;
    let pml = dir.join("canary.pml");
    std::fs::write(&pml, CANARY_MODEL).map_err(|e| format!("canary write: {e}"))?;
    for p in [dir.join("pan"), dir.join("pan.c")] {
        let _ = std::fs::remove_file(p);
    }
    let generated = std::process::Command::new("spin")
        .args(["-a", "canary.pml"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("canary spin: {e}"))?;
    if !generated.status.success() || !dir.join("pan.c").exists() {
        return Err("canary: spin could not generate a verifier".into());
    }
    let cc = std::process::Command::new("gcc")
        .args(["-w", "-o", "pan", "pan.c"])
        .stdin(std::process::Stdio::null())
        .current_dir(dir)
        .output()
        .map_err(|e| format!("canary gcc: {e}"))?;
    if !cc.status.success() {
        return Err("canary: gcc could not build the verifier".into());
    }
    // One run per claim: a default pan evaluates only the first never claim.
    for claim in &CANARY_CLAIMS {
        let mut args = vec!["-N", claim.label, "-n"];
        if claim.liveness {
            args.push("-a");
        }
        let out = std::process::Command::new(dir.join("pan"))
            .args(&args)
            .current_dir(dir)
            .output()
            .map_err(|e| format!("canary pan: {e}"))?;
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if super::spin::pan_found_error(&combined) != claim.must_be_refuted {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    #[test]
    fn canary_model_parses_for_the_in_process_engine() {
        assert!(spin_rs::verify(super::CANARY_MODEL).is_ok());
    }

    #[test]
    fn the_in_process_engine_answers_every_canary_claim_as_expected() {
        assert_eq!(
            super::calibrated(super::CheckerBackend::SpinRs),
            Ok(true),
            "the in-process backend must refute the false claims and leave the true one clean"
        );
    }

    #[test]
    fn the_external_engine_answers_every_canary_claim_as_expected_when_installed() {
        if std::process::Command::new("spin")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        assert_eq!(super::calibrated(super::CheckerBackend::Spin), Ok(true));
    }
}

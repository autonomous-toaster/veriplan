//! Calibration canary.
//!
//! A fixed model whose verdicts are known, run through the selected backend before any plan
//! result is trusted. The model carries one property of each shape the translator emits, and
//! every one of them is false in a reachable state, so a correct engine must refute all of them:
//!
//!   p0  [] ( !(ca && cb) )        safety: both flags can be 1.
//!   p1  [] ( cb -> <> (ca && !ca) )  liveness in the shape veriplan emits for IF...THEN: the
//!                                   consequent is never true.
//!
//! An engine that inverts liveness refutes p0 and reports p1 clean, and one that never explores
//! the model refutes neither. Either way the plan verdict is worthless. This is the one check
//! that does not depend on the engine being right about itself.

use super::CheckerBackend;

/// The canary model. Kept tiny so the check costs milliseconds.
pub const CANARY_MODEL: &str = r#"
bool ca = 0;
bool cb = 0;
active proctype CanaryA() { do :: (1) -> ca = 1; break od }
active proctype CanaryB() { do :: (1) -> cb = 1; break od }
ltl p0 { [] ( !(ca && cb) ) }
ltl p1 { [] ( cb -> <> (ca && !(ca)) ) }
"#;

/// Every claim the canary declares. All of them must be refuted.
const CANARY_CLAIMS: [&str; 2] = ["p0", "p1"];

/// Run the canary through a backend. Ok(true) when the backend refuted every claim.
pub(crate) fn refuted(backend: CheckerBackend) -> Result<bool, String> {
    match backend {
        CheckerBackend::SpinRs => {
            let result = spin_rs::verify(CANARY_MODEL)
                .map_err(|e| format!("canary could not run on spin-rs: {e}"))?;
            let refuted: Vec<&str> = result
                .violations
                .iter()
                .map(|v| v.property_name.as_str())
                .collect();
            Ok(CANARY_CLAIMS.iter().all(|c| refuted.contains(c)))
        }
        CheckerBackend::Spin => refuted_by_spin(),
    }
}

fn refuted_by_spin() -> Result<bool, String> {
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
    for claim in CANARY_CLAIMS {
        let mut args = vec!["-N", claim, "-n"];
        if claim == "p1" {
            args.push("-a"); // liveness claim
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
        if !super::spin::pan_found_error(&combined) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    #[test]
    fn canary_model_parses_for_the_in_process_engine() {
        // A canary that cannot even be parsed would silently 'pass'.
        assert!(spin_rs::verify(super::CANARY_MODEL).is_ok());
    }

    #[test]
    fn the_in_process_engine_refutes_every_canary_claim() {
        assert_eq!(
            super::refuted(super::CheckerBackend::SpinRs),
            Ok(true),
            "the in-process backend must refute both the safety and the liveness canary"
        );
    }

    #[test]
    fn the_external_engine_refutes_every_canary_claim_when_installed() {
        if std::process::Command::new("spin")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        assert_eq!(super::refuted(super::CheckerBackend::Spin), Ok(true));
    }
}

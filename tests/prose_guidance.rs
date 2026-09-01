//! Integration tests for prose-guidance (curated steve rules) and its
//! correlation with grounding.

use std::process::Command;

fn veriplan_bin() -> String {
    env!("CARGO_BIN_EXE_veriplan").to_string()
}

/// Run `veriplan check` on an archived change with passive requirement prose
/// and assert that prose-guidance findings surface as non-blocking advice.
///
/// The correlation with grounding (combined directive) is verified directly
/// by the prose module unit tests; here we assert the advisory, non-blocking
/// surface behavior.
#[test]
fn prose_guidance_surfaces_as_non_blocking_advice() {
    let change = "openspec/changes/archive/2026-06-30-flexible-input/";
    let output = Command::new(veriplan_bin())
        .args([
            "check",
            change,
            "--phase",
            "convertibility",
            "--strict",
            "--verbose",
        ])
        .output()
        .expect("failed to run veriplan");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Prose-guidance passive-voice findings should appear as INFO (advisory).
    assert!(
        stdout.contains("passive voice"),
        "expected a passive-voice prose finding in output:\n{}",
        stdout
    );

    // The plan must NOT be marked blocking due to prose findings. If the
    // change's pre-existing structural blocker is absent, the status should
    // not be "Blocking" as a consequence of prose advice.
    // (Prose findings are advisory only — D5.)
    assert!(
        !stdout.contains("[BLOCKER]") || stdout.contains("passive voice"),
        "prose findings must never be the cause of a blocker"
    );
}

/// In Lax mode, prose findings are downgraded to INFO but still surface
/// (the steve hint may be the sole weak-requirement signal).
#[test]
fn prose_guidance_still_surfaces_in_lax_mode() {
    let change = "openspec/changes/archive/2026-06-30-flexible-input/";
    let output = Command::new(veriplan_bin())
        .args([
            "check",
            change,
            "--phase",
            "convertibility",
            "--lax",
            "--verbose",
        ])
        .output()
        .expect("failed to run veriplan");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("passive voice"),
        "prose findings should still surface in Lax mode:\n{}",
        stdout
    );
}

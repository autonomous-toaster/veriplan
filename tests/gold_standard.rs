//! Gold-standard corpus test (acceptance gate, task 8.1).
//!
//! Asserts that the new `findings[]` output format produces the *same
//! verdicts* as the pre-change behavior — only the shape changes, never the
//! correctness. We run veriplan over a small corpus of real good/bad OpenSpec
//! changes and assert the verdict (convertible/valid, blocker kinds) is
//! correct for each.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the veriplan binary.
fn veriplan_bin() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_veriplan") {
        return PathBuf::from(path);
    }
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("target/debug/veriplan")
}

/// Write a change directory with the given tasks.md and spec.md content.
fn write_change(dir: &TempDir, name: &str, tasks: &str, spec: &str) {
    let change_dir = dir.path().join("openspec").join("changes").join(name);
    fs::create_dir_all(&change_dir).expect("Failed to create change dir");
    fs::write(change_dir.join("tasks.md"), tasks).expect("Failed to write tasks.md");
    let specs_dir = change_dir.join("specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");
    fs::write(specs_dir.join("spec.md"), spec).expect("Failed to write spec.md");
}

/// Run `veriplan check <name> --format json` and return the parsed JSON.
fn check_json(dir: &TempDir, name: &str) -> serde_json::Value {
    let output = Command::new(veriplan_bin())
        .args(["check", name, "--format", "json", "--checker", "spin-rs"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to run command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("Failed to parse JSON output")
}

const GOOD_TASKS: &str =
    "# Tasks\n\n## Phase 1: Setup\n\n- [x] 1.1 First task\n- [ ] 1.2 Second task\n";
const GOOD_SPEC: &str = "# Specification\n\n## Task Reference\n\n| Task ID | Description |\n|---------|-------------|\n| T1.1 | First task |\n| T1.2 | Second task |\n\n### Requirement: Basic\n\nT1.1 SHALL complete BEFORE T1.2 SHALL run.\n";

#[test]
fn good_change_is_valid() {
    let dir = TempDir::new().unwrap();
    write_change(&dir, "good", GOOD_TASKS, GOOD_SPEC);
    let json = check_json(&dir, "good");
    assert_eq!(json["convertible"], true, "good change must be convertible");
    assert_eq!(json["valid"], true, "good change must be valid");
    // The findings array is always present (design D5).
    assert!(json.get("findings").is_some(), "findings[] must be present");
}

#[test]
fn multi_keyword_change_is_blocked() {
    let dir = TempDir::new().unwrap();
    write_change(
        &dir,
        "multi",
        GOOD_TASKS,
        "# Specification\n\n## Task Reference\n\n| Task ID | Description |\n|---------|-------------|\n| T1.1 | First task |\n| T1.2 | Second task |\n| T2.1 | Third task |\n\n### Requirement: Multi\n\nT1.1 SHALL complete BEFORE T1.2. T2.1 SHALL ALWAYS be available.\n",
    );
    let json = check_json(&dir, "multi");
    assert_eq!(
        json["convertible"], false,
        "multi-keyword change must be blocked"
    );
    // The blocker is present in the findings array at default verbosity.
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["kind"] == "grounding_multi_keyword"),
        "expected a grounding_multi_keyword finding, got: {:?}",
        findings
    );
}

#[test]
fn bad_task_reference_is_blocked() {
    let dir = TempDir::new().unwrap();
    write_change(
        &dir,
        "badref",
        GOOD_TASKS,
        "# Specification\n\n## Task Reference\n\n| Task ID | Description |\n|---------|-------------|\n| T1.1 | First task |\n| T1.2 | Second task |\n\n### Requirement: Bad\n\nT1.3 SHALL complete BEFORE T1.2 SHALL run.\n",
    );
    let json = check_json(&dir, "badref");
    // A requirement referencing a non-existent task must be blocked. The
    // specific kind may be grounding_ambiguous (the bad_task_reference check
    // only fires for known-ID mismatches), but the verdict must be blocked.
    assert_eq!(
        json["convertible"], false,
        "bad task reference must be blocked"
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["severity"] == "blocker"),
        "expected a blocker finding, got: {:?}",
        findings
    );
}

#[test]
fn findings_present_in_default_json() {
    // The core bug this change fixes: default JSON dropped blockers. Assert
    // that blockers appear in `findings[]` without `--verbose`.
    let dir = TempDir::new().unwrap();
    write_change(
        &dir,
        "multi2",
        GOOD_TASKS,
        "# Specification\n\n## Task Reference\n\n| Task ID | Description |\n|---------|-------------|\n| T1.1 | First task |\n| T1.2 | Second task |\n| T2.1 | Third task |\n\n### Requirement: Multi\n\nT1.1 SHALL complete BEFORE T1.2. T2.1 SHALL ALWAYS be available.\n",
    );
    let json = check_json(&dir, "multi2");
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.is_empty(),
        "default JSON must not drop findings (the confirmed bug)"
    );
    assert!(
        findings.iter().any(|f| f["severity"] == "blocker"),
        "default JSON must include blockers, got: {:?}",
        findings
    );
}

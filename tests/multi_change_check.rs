//! Integration tests for multi-change check behavior.

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

/// Create a test project with the given changes.
fn setup_test_project(changes: &[&str]) -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let openspec_dir = dir.path().join("openspec");
    let changes_dir = openspec_dir.join("changes");

    fs::create_dir_all(&changes_dir).expect("Failed to create changes dir");

    for change in changes {
        let change_dir = changes_dir.join(change);
        fs::create_dir_all(&change_dir).expect("Failed to create change dir");

        // Create tasks.md
        fs::write(
            change_dir.join("tasks.md"),
            "# Tasks\n\n## Phase 1: Setup\n\n- [x] T1.1 Task\n",
        )
        .expect("Failed to write tasks.md");

        // Create specs/spec.md
        let specs_dir = change_dir.join("specs");
        fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");
        fs::write(
            specs_dir.join("spec.md"),
            "# Specification\n\n## Task Reference\n\n| Task ID | Description |\n|---------|-------------|\n| T1.1 | Setup task |\n\n### Requirement: Basic\n\nT1.1 SHALL complete BEFORE any other task runs.\n"
        ).expect("Failed to write spec.md");
    }

    dir
}

#[test]
fn test_zero_changes() {
    let dir = setup_test_project(&[]);
    let output = Command::new(veriplan_bin())
        .args(["check"])
        .current_dir(&dir)
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No active changes"),
        "Expected 'No active changes', got: {}",
        stdout
    );
}

#[test]
fn test_one_change() {
    let dir = setup_test_project(&["change-a"]);
    let output = Command::new(veriplan_bin())
        .args(["check"])
        .current_dir(&dir)
        .output()
        .expect("Failed to run command");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        combined.contains("change-a"),
        "Expected 'change-a' in output, got: {}",
        combined
    );
}

#[test]
fn test_two_valid_changes() {
    let dir = setup_test_project(&["change-a", "change-b"]);
    let output = Command::new(veriplan_bin())
        .args(["check"])
        .current_dir(&dir)
        .output()
        .expect("Failed to run command");

    assert!(
        output.status.code() == Some(0),
        "Expected exit 0, got {:?}. stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("All 2 changes valid"),
        "Expected success message, got: {}",
        stdout
    );
}

#[test]
fn test_mixed_validity() {
    let dir = setup_test_project(&["valid-change", "invalid-change"]);

    // Make invalid-change invalid by writing empty spec
    let invalid_specs_dir = dir.path().join("openspec/changes/invalid-change/specs");
    fs::write(
        invalid_specs_dir.join("spec.md"),
        "# Empty\n\nNo requirements.\n",
    )
    .expect("Failed to write invalid spec");

    let output = Command::new(veriplan_bin())
        .args(["check"])
        .current_dir(&dir)
        .output()
        .expect("Failed to run command");

    assert!(
        output.status.code() == Some(1),
        "Expected exit 1, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("changes invalid"),
        "Expected 'changes invalid', got: {}",
        stderr
    );
    assert!(
        stderr.contains("invalid-change"),
        "Expected 'invalid-change', got: {}",
        stderr
    );
}

#[test]
fn test_json_output() {
    let dir = setup_test_project(&["change-a", "change-b"]);
    let output = Command::new(veriplan_bin())
        .args(["check", "--format", "json"])
        .current_dir(&dir)
        .output()
        .expect("Failed to run command");

    assert!(
        output.status.code() == Some(0),
        "Expected exit 0, got {:?}. stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .expect("Failed to parse JSON");

    assert!(json.get("changes").is_some(), "Expected 'changes' field");
    assert!(
        json.get("all_valid").is_some(),
        "Expected 'all_valid' field"
    );
    assert!(
        json.get("invalid_changes").is_some(),
        "Expected 'invalid_changes' field"
    );
    assert_eq!(json["changes"].as_array().unwrap().len(), 2);
}

#[test]
fn test_explicit_change_name() {
    let dir = setup_test_project(&["change-a", "change-b"]);
    let output = Command::new(veriplan_bin())
        .args(["check", "change-a"])
        .current_dir(&dir)
        .output()
        .expect("Failed to run command");

    assert!(
        output.status.code() == Some(0),
        "Expected exit 0, got {:?}. stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("change-a"), "Expected 'change-a' in output");
    assert!(
        !stdout.contains("change-b"),
        "Did not expect 'change-b' in output"
    );
}

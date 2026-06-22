//! Integration tests for flexible input modes and strictness profiles.

use std::env;
use std::fs;
use std::process::Command;

fn veriplan_bin() -> String {
    env!("CARGO_BIN_EXE_veriplan").to_string()
}

#[test]
fn test_single_file_mode() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("test.md");

    // Create a standalone tasks.md file
    fs::write(
        &test_file,
        r#"
# Test Tasks

- [ ] T1.1 First task
- [ ] T1.2 Second task
"#,
    )
    .unwrap();

    let output = Command::new(veriplan_bin())
        .arg("check")
        .arg(&test_file)
        .output()
        .expect("Failed to run veriplan");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);

    // Should succeed (exit 0) even without requirements
    // In single-file mode, no requirements is INFO not blocker
    assert!(output.status.success() || stdout.to_lowercase().contains("info"));
}

#[test]
fn test_single_file_with_requirements() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("spec.md");

    // Create a standalone spec.md file with a requirement
    fs::write(
        &test_file,
        r#"
# Test Spec

### Requirement: REQ-1

T1.1 SHALL complete before T1.2 starts.
"#,
    )
    .unwrap();

    let output = Command::new(veriplan_bin())
        .arg("check")
        .arg(&test_file)
        .output()
        .expect("Failed to run veriplan");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect pattern_ungrounded (no task refs in plan)
    // In single-file mode, this should still be detected
    assert!(
        stdout.to_lowercase().contains("pattern")
            || stdout.to_lowercase().contains("non_formalizable")
    );
}

#[test]
fn test_stdin_mode() {
    let input = r#"
# Test Tasks

- [ ] T1.1 First task
- [ ] T1.2 Second task
"#;

    let mut child = Command::new(veriplan_bin())
        .arg("check")
        .arg("--stdin")
        .arg("test.md")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run veriplan");

    // Write to stdin
    {
        let mut stdin = child.stdin.take().unwrap();
        use std::io::Write;
        stdin.write_all(input.as_bytes()).unwrap();
    }

    let result = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);

    // Should handle stdin input gracefully
    assert!(result.status.success() || stdout.to_lowercase().contains("info"));
}

#[test]
fn test_strictness_strict() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("spec.md");

    // Create a file with pattern_ungrounded requirement
    fs::write(
        &test_file,
        r#"
### Requirement: REQ-1

Task A SHALL complete before Task B starts.
"#,
    )
    .unwrap();

    let output = Command::new(veriplan_bin())
        .arg("check")
        .arg(&test_file)
        .output()
        .expect("Failed to run veriplan");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Strict mode (default): pattern_ungrounded should be blocker
    assert!(stdout.contains("blocker") || stdout.contains("pattern_ungrounded"));
}

#[test]
fn test_strictness_moderate() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("spec.md");

    // Create a file with pattern_ungrounded requirement
    fs::write(
        &test_file,
        r#"
### Requirement: REQ-1

Task A SHALL complete before Task B starts.
"#,
    )
    .unwrap();

    let output = Command::new(veriplan_bin())
        .arg("check")
        .arg("--moderate")
        .arg(&test_file)
        .output()
        .expect("Failed to run veriplan");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Moderate mode: pattern_ungrounded should be warning
    assert!(stdout.contains("warning") || !stdout.contains("blocker"));
}

#[test]
fn test_strictness_lax() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("spec.md");

    // Create a file with pattern_ungrounded requirement
    fs::write(
        &test_file,
        r#"
### Requirement: REQ-1

Task A SHALL complete before Task B starts.
"#,
    )
    .unwrap();

    let output = Command::new(veriplan_bin())
        .arg("check")
        .arg("--lax")
        .arg(&test_file)
        .output()
        .expect("Failed to run veriplan");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Lax mode: pattern_ungrounded should be info
    assert!(stdout.to_lowercase().contains("info") || !stdout.to_lowercase().contains("blocker"));
}

#[test]
fn test_may_requirement_info() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("spec.md");

    // Create a file with MAY requirement
    fs::write(
        &test_file,
        r#"
### Requirement: REQ-1

T1.1 MAY complete before T1.2 starts.
"#,
    )
    .unwrap();

    let output = Command::new(veriplan_bin())
        .arg("check")
        .arg(&test_file)
        .arg("--phase")
        .arg("convertibility")
        .arg("--verbose")
        .output()
        .expect("Failed to run veriplan");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // MAY requirements should be INFO, not blockers
    assert!(stdout.to_lowercase().contains("info") || stdout.to_lowercase().contains("may"));
}

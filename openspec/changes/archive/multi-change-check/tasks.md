# Tasks

## Phase 1: Input Resolution

### 1.1 Add InputSource::MultiOpenSpec variant

**Status:** Complete

Add new enum variant to `src/input/mod.rs`:

```rust
pub enum InputSource {
    OpenSpec { change_dir: PathBuf, change_name: String },
    Directory { path: PathBuf, has_tasks: bool, has_specs: bool },
    SingleFile { path: PathBuf },
    Stdin { content: String, label: String },
    
    // NEW
    MultiOpenSpec { 
        changes: Vec<String>,
        project_root: PathBuf,
    },
}
```

**Dependencies:** None

---

### 1.2 Update resolve_auto() to return MultiOpenSpec

**Status:** Complete

Modify `resolve_auto()` in `src/input/mod.rs`:

```rust
fn resolve_auto(project_root: &Path) -> Result<InputSource, String> {
    let changes_dir = project_root.join("openspec").join("changes");
    if changes_dir.exists() && changes_dir.is_dir() {
        let changes = discover_changes(project_root)?;
        match changes.len() {
            0 => Err(format!("No active changes found in {}", changes_dir.display())),
            1 => Ok(InputSource::OpenSpec {
                change_dir: changes_dir.join(&changes[0]),
                change_name: changes[0].clone(),
            }),
            _ => Ok(InputSource::MultiOpenSpec { 
                changes, 
                project_root: project_root.to_path_buf(),
            }),  // NEW: not an error anymore
        }
    } else {
        // ... existing logic
    }
}
```

**Dependencies:** T1.1

---

## Phase 2: Command Handler

### 1.3 Update run_check() to handle MultiOpenSpec

**Status:** Complete

Modify `run_check()` in `src/main.rs`:

```rust
fn run_check(change_name: Option<String>, ...) -> Result<()> {
    let project_root = std::env::current_dir()?;
    
    let source = match change_name {
        None => resolve_auto(&project_root)?,  // Can return MultiOpenSpec
        Some(name) => resolve_input(Some(name), &project_root, false)?,
    };
    
    match source {
        InputSource::MultiOpenSpec { changes, project_root } => {
            check_all_changes(&changes, &project_root, format, verbose, pre_commit, strictness)?;
        }
        _ => {
            // Existing single-change logic
            let plan = load_plan(&source)?;
            let result = checker::verify_with_strictness(...);
            // ... existing output logic
        }
    }
}
```

**Dependencies:** T1.1, T1.2

---

### 1.4 Implement check_all_changes() function

**Status:** Complete

Add new function in `src/main.rs`:

```rust
fn check_all_changes(
    changes: &[String],
    project_root: &Path,
    format: &str,
    verbose: bool,
    pre_commit: bool,
    strictness: StrictnessProfile,
) -> Result<()> {
    let mut results = Vec::new();
    
    for change in changes {
        let source = InputSource::OpenSpec {
            change_dir: project_root.join("openspec/changes").join(change),
            change_name: change.clone(),
        };
        
        let plan = load_plan(&source)?;
        let result = checker::verify_with_strictness(
            &plan,
            change,
            false, // no_model
            pre_commit,
            strictness,
            true, // is_openspec
        );
        results.push((change.clone(), result));
    }
    
    match format {
        "json" => print_multi_json(&results),
        _ => print_multi_human(&results, verbose),
    }
    
    // Exit code
    if results.iter().all(|(_, r)| r.valid.unwrap_or(false)) {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
```

**Dependencies:** T1.3

---

## Phase 3: Output Formatting

### 1.5 Implement human-readable multi-change output

**Status:** Complete

Add function in `src/main.rs` or `src/annotator/mod.rs`:

```rust
fn print_multi_human(results: &[(String, VerificationResult)], verbose: bool) {
    let total = results.len();
    let invalid: Vec<_> = results.iter()
        .filter(|(_, r)| !r.valid.unwrap_or(false))
        .collect();
    
    if invalid.is_empty() {
        println!("✓ All {} changes valid", total);
    } else {
        eprintln!("✗ {}/{} changes invalid", invalid.len(), total);
        for (name, _) in &invalid {
            eprintln!("  - {}: INVALID", name);
        }
        eprintln!();
        eprintln!("Run:");
        for (name, _) in &invalid {
            eprintln!("  veriplan check {}", name);
        }
    }
}
```

**Dependencies:** T1.4

---

### 1.6 Implement JSON multi-change output

**Status:** Complete

Add function in `src/main.rs` or `src/annotator/mod.rs`:

```rust
fn print_multi_json(results: &[(String, VerificationResult)]) {
    let mut changes_json = Vec::new();
    let mut invalid_changes = Vec::new();
    
    for (name, result) in results {
        changes_json.push(serde_json::json!({
            "name": name,
            "valid": result.valid,
            "plan_name": result.plan_name,
        }));
        
        if !result.valid.unwrap_or(false) {
            invalid_changes.push(name);
        }
    }
    
    let output = serde_json::json!({
        "changes": changes_json,
        "all_valid": invalid_changes.is_empty(),
        "invalid_changes": invalid_changes,
    });
    
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
```

**Dependencies:** T1.4

---

## Phase 4: Testing

### 1.7 Add integration tests for multi-change behavior

**Status:** Complete

Create test file `tests/multi_change_check.rs`:

```rust
#[cfg(test)]
mod tests {
    use std::process::Command;
    use tempfile::TempDir;
    
    fn setup_test_project(changes: &[&str]) -> TempDir {
        // Create temp dir with openspec/changes/ structure
        // Return temp dir (auto-cleanup)
    }
    
    #[test]
    fn test_zero_changes() {
        let dir = setup_test_project(&[]);
        let output = Command::new("cargo")
            .args(["run", "--", "check"])
            .current_dir(&dir)
            .output()
            .unwrap();
        
        assert!(output.status.code() == Some(2)); // Error exit
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("No active changes"));
    }
    
    #[test]
    fn test_one_change() {
        // Existing behavior - should work as before
    }
    
    #[test]
    fn test_two_valid_changes() {
        let dir = setup_test_project(&["change-a", "change-b"]);
        let output = Command::new("cargo")
            .args(["run", "--", "check"])
            .current_dir(&dir)
            .output()
            .unwrap();
        
        assert!(output.status.code() == Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("All 2 changes valid"));
    }
    
    #[test]
    fn test_mixed_validity() {
        // One valid, one invalid
        // Check output format and exit code
    }
    
    #[test]
    fn test_json_output() {
        // Test --format json with multiple changes
        // Validate JSON structure
    }
}
```

**Dependencies:** T1.4, T1.5, T1.6

---

## Acceptance Criteria

- [x] `veriplan check` with 0 changes → error "No active changes"
- [x] `veriplan check` with 1 change → full output (unchanged)
- [x] `veriplan check` with 2+ changes → concise summary
- [x] Concise summary shows only invalid changes
- [x] Concise summary includes commands to dig deeper
- [x] Exit code 0 when all valid, 1 when any invalid
- [x] JSON output wraps in `{"changes": [...]}` for multi-change
- [x] Temp files don't conflict (change-name isolation)
- [x] Explicit change name still works: `veriplan check change-a`
- [x] All integration tests pass

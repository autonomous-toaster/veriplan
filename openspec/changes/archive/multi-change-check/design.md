# Design

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    SYSTEM ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐  │
│  │  CLI Layer   │      │ Input Layer  │      │ Checker Layer│  │
│  │  (main.rs)   │◀────▶│ (input/mod)  │◀────▶│ (checker/mod)│  │
│  │              │      │              │      │              │  │
│  │ run_check()  │      │ resolve_*()  │      │ verify()     │  │
│  │              │      │ load_plan()  │      │              │  │
│  └──────────────┘      └──────────────┘      └──────────────┘  │
│         │                      │                      │         │
│         │                      │                      │         │
│         ▼                      ▼                      ▼         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Output Layer (annotator/mod.rs)             │  │
│  │                                                          │  │
│  │  format_human()  format_json()  print_multi_*()         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Single Change (Existing)

```
veriplan check
     │
     ▼
resolve_auto() → InputSource::OpenSpec { change_name, change_dir }
     │
     ▼
load_plan() → PlanIR
     │
     ▼
checker::verify() → VerificationResult
     │
     ▼
print_human() / print_json()
```

### Multiple Changes (New)

```
veriplan check (no args, 2+ changes)
     │
     ▼
resolve_auto() → InputSource::MultiOpenSpec { changes: Vec<String>, project_root }
     │
     ▼
check_all_changes()
     │
     ├─→ [Loop for each change]
     │      │
     │      ├─→ InputSource::OpenSpec { ... }
     │      │
     │      ├─→ load_plan() → PlanIR
     │      │
     │      └─→ checker::verify() → VerificationResult
     │
     ▼
print_multi_human() / print_multi_json()
```

## Component Design

### 1. InputSource Enum Extension

**Location:** `src/input/mod.rs`

```rust
pub enum InputSource {
    /// OpenSpec change directory (single)
    OpenSpec {
        change_dir: PathBuf,
        change_name: String,
    },
    /// Loose directory with tasks.md and/or specs/
    Directory {
        path: PathBuf,
        has_tasks: bool,
        has_specs: bool,
    },
    /// Single .md file
    SingleFile { path: PathBuf },
    /// Content from stdin
    Stdin { content: String, label: String },
    /// NEW: Multiple OpenSpec changes detected
    MultiOpenSpec {
        changes: Vec<String>,
        project_root: PathBuf,
    },
}
```

**Rationale:**

- Keeps multi-change logic within existing input resolution system
- Allows pattern matching in command handler
- Clean separation from single-change flow

### 2. resolve_auto() Modification

**Location:** `src/input/mod.rs`

**Current:**

```rust
match changes.len() {
    0 => Err("No active changes"),
    1 => Ok(single),
    _ => Err("Multiple active changes"),  // ← ERROR
}
```

**New:**

```rust
match changes.len() {
    0 => Err("No active changes"),
    1 => Ok(single),
    _ => Ok(MultiOpenSpec { changes, project_root }),  // ← SUCCESS
}
```

**Rationale:**

- Multiple changes is a valid state, not an error
- Delegates decision to command handler
- Maintains backward compatibility for explicit args

### 3. run_check() Pattern Match

**Location:** `src/main.rs`

```rust
fn run_check(change_name: Option<String>, ...) -> Result<()> {
    let project_root = std::env::current_dir()?;
    
    let source = match change_name {
        None => resolve_auto(&project_root)?,
        Some(name) => resolve_input(Some(name), &project_root, false)?,
    };
    
    match source {
        InputSource::MultiOpenSpec { changes, project_root } => {
            // NEW: delegate to multi-change handler
            check_all_changes(&changes, &project_root, format, verbose, pre_commit, strictness)
        }
        _ => {
            // EXISTING: single-change logic
            let plan = load_plan(&source)?;
            let result = checker::verify_with_strictness(...);
            print_single_result(result, format, verbose);
        }
    }
}
```

**Rationale:**

- Clean separation of concerns
- No changes to existing single-change path
- Easy to test independently

### 4. check_all_changes() Implementation

**Location:** `src/main.rs`

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
    
    // Sequential checking (not parallel)
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
    
    // Output formatting
    match format {
        "json" => print_multi_json(&results),
        _ => print_multi_human(&results, verbose),
    }
    
    // Exit code based on results
    if results.iter().all(|(_, r)| r.valid.unwrap_or(false)) {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
```

**Rationale:**

- Sequential for simplicity and ordered output
- Reuses existing `checker::verify_with_strictness()`
- Aggregates results for summary output

### 5. Output Functions

**Location:** `src/main.rs` or `src/annotator/mod.rs`

#### Human-Readable Output

```rust
fn print_multi_human(results: &[(String, VerificationResult)], verbose: bool) {
    let total = results.len();
    let invalid: Vec<_> = results.iter()
        .filter(|(_, r)| !r.valid.unwrap_or(false))
        .collect();
    
    if invalid.is_empty() {
        // All valid - concise success
        println!("✓ All {} changes valid", total);
    } else {
        // Some invalid - show only invalid, commands to dig deeper
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

#### JSON Output

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

**Rationale:**

- Human output: concise, actionable, commands included
- JSON output: machine-parseable, structured
- Both support AI assistant integration

## Temp File Isolation

**SPIN temp file pattern:** `/tmp/veriplan_{change_name}.pml`

```
Change "change-a" → /tmp/veriplan_change-a.pml
Change "change-b" → /tmp/veriplan_change-b.pml
Change "change-c" → /tmp/veriplan_change-c.pml
```

**No conflicts** because:

- Each change gets unique filename based on change name
- Sequential execution (not parallel)
- Temp files cleaned up after each check

## Error Handling

### Error Scenarios

| Scenario | Behavior |
|----------|----------|
| 0 changes | Error: "No active changes found" (exit 2) |
| 1 change | Full output (existing behavior) |
| 2+ changes, all valid | Concise success (exit 0) |
| 2+ changes, some invalid | Concise failure with commands (exit 1) |
| Parse error on one change | Error message for that change, continue with others |
| SPIN not found | Warning, skip model check for that change |

### Partial Failures

If one change fails to parse or check:

- Continue checking remaining changes
- Report error for failed change in summary
- Exit code reflects overall status

## Backward Compatibility

### Preserved Behaviors

| Behavior | Status |
|----------|--------|
| `veriplan check change-name` | ✅ Unchanged - single change |
| `veriplan check` with 1 change | ✅ Unchanged - full output |
| `veriplan check --format json` | ✅ Unchanged for single change |
| Exit codes for single change | ✅ Unchanged |

### Changed Behaviors

| Behavior | Before | After |
|----------|--------|-------|
| `veriplan check` with 2+ changes | Error | Check all, summary |
| Output format for 2+ changes | N/A | Concise |
| JSON structure for 2+ changes | N/A | Wrapped in array |

## Testing Strategy

### Unit Tests

- `InputSource::MultiOpenSpec` variant exists
- `resolve_auto()` returns correct variant for 0/1/2+ changes
- `check_all_changes()` aggregates results correctly

### Integration Tests

- 0 changes → error message
- 1 change → full output (unchanged)
- 2 changes, both valid → concise success
- 2 changes, one invalid → concise failure
- JSON output structure
- Exit codes

### Manual Testing

```bash
# Test in veriplan repo itself
cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/veriplan
cargo run -- check

# Test with multiple changes
# (create test changes if needed)
```

## Implementation Notes

### Dependencies

- No new external dependencies
- Uses existing `serde_json` for JSON output
- Uses existing `checker::verify_with_strictness()`

### Performance

- Sequential checking: O(n) where n = number of changes
- Typical case: 2-3 changes → negligible impact
- No parallelization needed for typical workloads

### Future Enhancements

- Parallel checking with `tokio` for 10+ changes
- Heuristic to skip old/stale changes
- `--all` flag to force multi-check even with 1 change
- `--summary` flag for concise output on single change

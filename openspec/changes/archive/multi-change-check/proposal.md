# Multi-Change Auto-Check

## Problem Statement

When running `veriplan check` with no arguments in a project with multiple active OpenSpec changes, the CLI currently errors with:

```
Error: Multiple active changes found. Specify one: [change-a, change-b, change-c]
```

This forces users (and AI assistants) to:

1. Manually identify which changes exist
2. Run `veriplan check` on each one individually
3. Parse multiple outputs to understand overall status

**This is especially problematic for AI coding assistants** that need to:

- Detect validation issues across all active work
- Provide actionable feedback without bloating context
- Know the exact commands to run for deeper inspection

## Current Behavior

```
┌─────────────────────────────────────────────────────────┐
│  veriplan check (no args, multiple changes)            │
│         │                                                │
│         ▼                                                │
│  resolve_auto() detects 2+ changes                     │
│         │                                                │
│         ▼                                                │
│  Returns Error("Multiple active changes found...")     │
│         │                                                │
│         ▼                                                │
│  User must run:                                         │
│    veriplan check change-a                              │
│    veriplan check change-b                              │
│    veriplan check change-c                              │
└─────────────────────────────────────────────────────────┘
```

## Proposed Behavior

```
┌─────────────────────────────────────────────────────────┐
│  veriplan check (no args, multiple changes)            │
│         │                                                │
│         ▼                                                │
│  resolve_auto() returns MultiOpenSpec source           │
│         │                                                │
│         ▼                                                │
│  check_all_changes() runs on each sequentially         │
│         │                                                │
│         ▼                                                │
│  Concise summary output:                                │
│    "✗ 2/3 changes invalid"                              │
│    "  - change-b: INVALID"                              │
│    "  - change-c: INVALID"                              │
│    "Run: veriplan check change-b"                       │
│          "veriplan check change-c"                      │
│         │                                                │
│         ▼                                                │
│  AI assistant can:                                      │
│    - Parse output (human or JSON)                       │
│    - Run detailed checks on invalid changes             │
│    - Present focused feedback to user                   │
└─────────────────────────────────────────────────────────┘
```

## Non-Goals

- Changing how individual change checks work
- Auto-archiving or managing OpenSpec workflow
- Parallel checking (sequential is fine for 2-3 changes)
- Requiring flags or opt-in (this is default behavior)

## Solution Overview

### 1. Input Resolution Layer

Add new `InputSource` variant for multiple changes:

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

### 2. Auto-Detection Logic

In `resolve_auto()`:

```rust
match changes.len() {
    0 => Err("No active changes found"),
    1 => Ok(InputSource::OpenSpec { ... }),
    _ => Ok(InputSource::MultiOpenSpec { changes, project_root }),  // NEW
}
```

### 3. Command Handler

In `run_check()`:

```rust
let source = match change_name {
    None => resolve_auto(&project_root)?,  // Can return MultiOpenSpec
    Some(name) => resolve_input(Some(name), ...)?,  // Single only
};

match source {
    InputSource::MultiOpenSpec { changes, project_root } => {
        check_all_changes(&changes, &project_root, ...)?;
    }
    _ => { /* existing single-change logic */ }
}
```

### 4. Multi-Change Execution

```rust
fn check_all_changes(
    changes: &[String],
    project_root: &Path,
    format: &str,
    verbose: bool,
) -> Result<()> {
    let mut results = Vec::new();
    
    for change in changes {
        let source = InputSource::OpenSpec {
            change_dir: project_root.join("openspec/changes").join(change),
            change_name: change.clone(),
        };
        
        let plan = load_plan(&source)?;
        let result = checker::verify(&plan, ...);
        results.push((change.clone(), result));
    }
    
    match format {
        "json" => print_multi_json(&results),
        _ => print_multi_human(&results),
    }
    
    // Exit 0 if all valid, 1 if any invalid
    if results.iter().all(|(_, r)| r.valid.unwrap_or(false)) {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
```

### 5. Output Formats

**Human-readable (default):**

```
All valid:
  ✓ All 3 changes valid

Some invalid:
  ✗ 2/3 changes invalid
    - change-b: INVALID
    - change-c: INVALID
  
  Run: veriplan check change-b
       veriplan check change-c
```

**JSON (--format json):**

```json
{
  "changes": [
    {"name": "change-a", "valid": true, "plan_name": "change-a"},
    {"name": "change-b", "valid": false, "plan_name": "change-b"},
    {"name": "change-c", "valid": false, "plan_name": "change-c"}
  ],
  "all_valid": false,
  "invalid_changes": ["change-b", "change-c"]
}
```

## Technical Considerations

### Temp File Isolation

SPIN model checker generates temp files at `/tmp/veriplan_{change_name}.pml`. Each change gets unique temp files based on change name, so **no conflicts** when checking multiple changes sequentially.

### Batch Size

For typical workflows (2-3 active changes), sequential checking is fine. If 10+ changes exist, still check all - user likely knows why. Can add heuristics later if needed.

### Backward Compatibility

- Explicit change name arg (`veriplan check change-a`) still checks single change
- Single change auto-detect still shows full output
- Only affects no-arg case with 2+ changes

## Tasks

See tasks.md for implementation breakdown.

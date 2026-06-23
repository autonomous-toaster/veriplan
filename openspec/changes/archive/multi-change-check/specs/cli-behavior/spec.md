# CLI Multi-Change Behavior

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add InputSource::MultiOpenSpec variant |
| T1.2 | Update resolve_auto() to return MultiOpenSpec |
| T1.3 | Update run_check() to handle MultiOpenSpec |
| T1.4 | Implement check_all_changes() function |
| T1.5 | Implement human-readable multi-change output |
| T1.6 | Implement JSON multi-change output |
| T1.7 | Add integration tests for multi-change behavior |

## Requirements

### R1 - Input Source Extension

**T1.1 SHALL** add a new `InputSource::MultiOpenSpec` variant to the `InputSource` enum in `src/input/mod.rs`.

The variant **SHALL** contain:

- `changes: Vec<String>` - list of change names
- `project_root: PathBuf` - project root path

This requirement **SHALL** be satisfied **BEFORE** T1.2 runs.

### R2 - Auto-Detection Logic

**T1.2 SHALL** modify `resolve_auto()` in `src/input/mod.rs` to handle the multiple changes case.

When `discover_changes()` returns 2 or more changes:

- **T1.2 SHALL** return `Ok(InputSource::MultiOpenSpec { changes, project_root })`
- **T1.2 SHALL NOT** return an error

When `discover_changes()` returns 0 changes:

- **T1.2 SHALL** return `Err("No active changes found in ...")`

When `discover_changes()` returns 1 change:

- **T1.2 SHALL** return `Ok(InputSource::OpenSpec { ... })` (existing behavior)

This requirement **SHALL** be satisfied **BEFORE** T1.3 runs.

### R3 - Command Handler Update

**T1.3 SHALL** modify `run_check()` in `src/main.rs` to pattern match on `InputSource::MultiOpenSpec`.

**WHEN** `InputSource::MultiOpenSpec` is received:

- **T1.3 SHALL** call `check_all_changes()` with the change list
- **T1.3 SHALL NOT** attempt to load a single plan

**WHEN** any other `InputSource` variant is received:

- **T1.3 SHALL** use existing single-change logic

This requirement **SHALL** be satisfied **BEFORE** T1.4 runs.

### R4 - Multi-Change Execution

**T1.4 SHALL** implement `check_all_changes()` function in `src/main.rs`.

**T1.4 SHALL** for each change name:

- Create an `InputSource::OpenSpec` with the change directory
- Call `load_plan()` to parse the change
- Call `checker::verify_with_strictness()` to validate
- Store the result with the change name

**T1.4 SHALL** after checking all changes:

- Call output formatting function based on `--format` flag
- Exit with code 0 if all changes valid
- Exit with code 1 if any change invalid

**T1.4 SHALL** process changes sequentially (not in parallel).

This requirement **SHALL** be satisfied **CONCURRENTLY** with T1.5 and T1.6.

### R5 - Human-Readable Output

**T1.5 SHALL** implement `print_multi_human()` function for human-readable output.

**WHEN** all changes are valid:

- **T1.5 SHALL** print: `✓ All N changes valid`
- **T1.5 SHALL** exit with code 0

**WHEN** some changes are invalid:

- **T1.5 SHALL** print: `✗ M/N changes invalid`
- **T1.5 SHALL** list each invalid change: `- change-name: INVALID`
- **T1.5 SHALL** print commands to dig deeper:

  ```
  Run: veriplan check change-b
       veriplan check change-c
  ```

- **T1.5 SHALL** exit with code 1

**WHEN** all changes are invalid:

- **T1.5 SHALL** list all changes as invalid
- **T1.5 SHALL** print commands for all changes
- **T1.5 SHALL** exit with code 1

This requirement **SHALL** be satisfied **CONCURRENTLY** with T1.6.

### R6 - JSON Output

**T1.6 SHALL** implement `print_multi_json()` function for machine-readable output.

**WHEN** `--format json` is specified with multiple changes:

- **T1.6 SHALL** output a JSON object with structure:

  ```json
  {
    "changes": [
      {
        "name": "change-a",
        "valid": true,
        "plan_name": "change-a"
      }
    ],
    "all_valid": false,
    "invalid_changes": ["change-b", "change-c"]
  }
  ```

**T1.6 SHALL** include for each change:

- `name`: the change name
- `valid`: boolean (true/false/null if not convertible)
- `plan_name`: same as name (for consistency with single-change format)

This requirement **SHALL** be satisfied **BEFORE** T1.7 runs.

### R7 - Integration Tests

**T1.7 SHALL** add integration tests in `tests/` directory for multi-change behavior.

**T1.7 SHALL** test the following scenarios:

1. **T1.7.1 SHALL** test 0 changes → error message
2. **T1.7.2 SHALL** test 1 change → full output (existing behavior unchanged)
3. **T1.7.3 SHALL** test 2 changes, both valid → concise success
4. **T1.7.4 SHALL** test 2 changes, one invalid → concise failure with commands
5. **T1.7.5 SHALL** test 2 changes, both invalid → concise failure with all commands
6. **T1.7.6 SHALL** test JSON output format with multiple changes
7. **T1.7.7 SHALL** test exit codes (0 for all valid, 1 for any invalid)

This requirement **SHALL** be satisfied **AFTER** T1.4, T1.5, and T1.6.

## Scenarios

### Scenario S1 - Multiple Changes Auto-Check

**GIVEN** a project with 3 active changes (change-a, change-b, change-c)

**WHEN** user runs `veriplan check` (no arguments)

**THEN**:

- All 3 changes are checked sequentially
- Concise summary is printed
- Exit code reflects overall validity

### Scenario S2 - AI Assistant Integration

**GIVEN** an AI assistant running in a project with multiple changes

**WHEN** assistant runs `veriplan check` and parses JSON output

**THEN**:

- Assistant receives structured data with all change statuses
- Assistant can identify invalid changes programmatically
- Assistant can run `veriplan check <change-name>` for details
- Assistant can present focused feedback without context bloat

### Scenario S3 - Explicit Change Name

**GIVEN** a project with multiple active changes

**WHEN** user runs `veriplan check change-b` (explicit name)

**THEN**:

- Only change-b is checked
- Full output is shown (not concise summary)
- Existing single-change behavior is preserved

## Constraints

### C1 - Sequential Execution

Multi-change checking **SHALL** be sequential, not parallel. This simplifies implementation and output ordering. Typical workflows have 2-3 changes, so performance impact is minimal.

### C2 - Temp File Isolation

Each change's SPIN verification uses `/tmp/veriplan_{change_name}.pml`. Change names **SHALL** be used to ensure temp file isolation. No additional locking or coordination is needed.

### C3 - Backward Compatibility

Explicit change name arguments **SHALL** continue to work as before. Only the no-argument auto-detect case **SHALL** change behavior when 2+ changes exist.

### C4 - No New Dependencies

The implementation **SHALL** use only existing Rust dependencies. No new crates or external libraries are needed.

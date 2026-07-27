## Context

The check goes at the very top of `main()`, before `Cli::parse()`. This ensures veriplan is disabled even if CLI arguments are invalid — the tool is completely silenced.

## Goals / Non-Goals

**Goals:**
- `VERIPLAN_DISABLE=1 veriplan check` prints a warning and exits 0
- Falsy values (`0`, `false`, `no`, empty) do NOT disable
- Works for all subcommands (check, init, visualize, lsp)

**Non-Goals:**
- Not a pre-commit hook skip mechanism (that's `VERIPLAN_SKIP`, mentioned in error messages but implemented in the hook script)
- No config file or persistent disable

## Decisions

**Decision 1: Check before Cli::parse()**
Before arg parsing means even `veriplan --help` is suppressed. This is intentional — when disabled, the user wants silence, not documentation.

**Decision 2: Truthy semantics**
Only truthy values disable: `1`, `true`, `yes`, or any non-empty non-falsy string. Falsy values (`0`, `false`, `no`, empty) are treated as "not disabled". This matches shell convention and allows `VERIPLAN_DISABLE=0` for explicit re-enable.

```rust
fn is_disabled() -> bool {
    match std::env::var("VERIPLAN_DISABLE").as_deref() {
        Ok(v) => !matches!(v, "0" | "false" | "no" | ""),
        Err(_) => false,
    }
}
```

**Decision 3: Warning to stderr, exit 0**
Stderr so it doesn't interfere with JSON output pipelines. Exit 0 so CI doesn't fail.

## Risks / Trade-offs

- **[Risk] User sets VERIPLAN_DISABLE accidentally**: Unlikely — environment variables are explicit. → **Mitigation**: Warning message makes it obvious.
- **[Risk] Forgetting to unset after debugging**: The warning on stderr is visible in CI logs. → **Mitigation**: None needed — the warning is the reminder.

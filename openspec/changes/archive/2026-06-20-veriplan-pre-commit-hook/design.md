# Design: veriplan pre-commit hook

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│         veriplan check --pre-commit                      │
│                                                           │
│  ┌──────────────┐     ┌──────────────┐                    │
│  │ Convertibility│────▶│  SPIN check   │                   │
│  │   (~10ms)      │     │  (if available)│                   │
│  └──────┬───────┘     └──────┬───────┘                    │
│         │                     │                             │
│  blockers?              violations?  no SPIN?              │
│  yes → exit 1           yes → exit 1  warn, exit 0        │
│  no → continue           no → exit 0                      │
│                                                           │
│  warnings → always exit 0                                │
│  missing SPIN → exit 0 with stderr warning                │
│                                                           │
│  Also: detect $PRE_COMMIT=1 for concise output mode       │
└─────────────────────────────────────────────────────────┘
```

## Exit code mapping

| Condition | `veriplan check` | `veriplan check --pre-commit` |
|---|---|---|
| All valid | 0 | 0 |
| Convertibility blockers | 2 | 1 |
| SPIN violations | 1 | 1 |
| SPIN not found | 2 | 0 (with warning) |
| Warnings only | 0 | 0 |

## Key decisions

1. **Full check by default** — not just convertibility. A convertible plan isn't necessarily valid. The whole point is catching violations that only SPIN finds.

2. **SPIN missing = graceful fallback** — don't block commits just because SPIN isn't installed. Print a clear warning and proceed.

3. **Check all active changes** — not just staged files. The plan is consistent as a whole; partial checks miss cross-constraint violations.

4. **Warnings don't block** — advisory findings (constraint diversity, unreferenced tasks) should inform, not block.

5. **`$PRE_COMMIT` env var** — when set by the pre-commit framework, format output more concisely (less verbose than `--verbose`, more actionable than default).

6. **Two hook IDs** — `veriplan` (language: rust, compiles from source) and `veriplan-system` (language: system, assumes installed). Teams choose based on their workflow.

7. **`pass_filenames: false`** — veriplan auto-detects which changes to check; filenames from pre-commit are irrelevant.

8. **`files: 'openspec/'`** — only triggers when openspec/ files are staged. Other commits skip the hook entirely.

## Output format in pre-commit mode

```
veriplan: checking 2 changes...
✓ my-feature — VALID (14/14)
✗ other-feature — BLOCKED: 2 blockers

  Blocker: T9.9 referenced in "T9.9 SHALL complete BEFORE T8.1" does not exist
  Blocker: requirement "Performance" has no temporal keyword

Commit blocked. Fix blockers above, or skip with: VERIPLAN_SKIP=1 git commit
```

When SPIN is missing:

```
veriplan: checking 1 change...
⚠ SPIN not found — skipping model checking. Install for full verification.
✓ my-feature — CONVERTIBLE (8/8 constraints pass convertibility)
```

## Implementation plan

### 1. Add `--pre-commit` flag to `check` subcommand

In `src/main.rs`, add a `--pre-commit` flag that:

- Sets a `pre_commit: bool` field on the `Check` variant
- Passes it through to the check logic

### 2. Modify exit code behavior

In the check dispatch, when `pre_commit` is true:

- Convert exit code 2 (not convertible / missing SPIN) to:
  - Exit 1 if there are actual blockers (convertibility failures)
  - Exit 0 with warning if SPIN is missing but convertibility passed
- Warnings always result in exit 0

### 3. Modify SPIN-missing behavior

In `src/checker/mod.rs`, `require_spin()` currently returns a hard error. In pre-commit mode, it should:

- Print a warning to stderr
- Return a soft result that allows the check to exit 0

### 4. Create `.pre-commit-hooks.yaml`

Two hook entries in the repo root.

### 5. README documentation

New section on pre-commit integration with setup instructions and explanation of what the hook checks.

## Files to modify

- `src/main.rs` — add `--pre-commit` flag, modify exit code logic
- `src/checker/mod.rs` — add pre-commit mode to `require_spin()` and verification flow
- `.pre-commit-hooks.yaml` — new file, two hook IDs
- `README.md` — new section on pre-commit integration

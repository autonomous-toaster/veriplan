## Why

The grounding check (integrated in the previous change) produces a false positive for MAY requirements. When a requirement uses "MAY" instead of "SHALL"/"MUST", the grounder's `extract_shall()` function doesn't find a SHALL/MUST sentence and falls back to the full body text. Without a temporal predicate keyword (BEFORE, AFTER, etc.), the RuleGrounder returns Ungroundable — even when the text contains valid task IDs like T6.7.

Additionally, the error message "no matching task or predicate found" is misleading when task IDs ARE present but the predicate keyword is missing.

## What Changes

- Skip MAY requirements in the grounding check — they are informational and don't need grounding
- Improve the ungroundable error message to distinguish "no predicate keyword" from "no task ID"
- Update the grounding-check spec to reflect MAY requirement handling

## Capabilities

### New Capabilities

*(none)*

### Modified Capabilities

- `grounding-check`: Add MAY requirement skipping; improve error message for predicate-missing vs task-missing cases

## Impact

- **Modified module**: `src/grounding/mod.rs` — skip MAY requirements, improve error messages
- **No breaking changes**: Behavior change only for MAY requirements (previously false positive blocker, now silently skipped)
